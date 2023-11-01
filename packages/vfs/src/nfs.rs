use crate::nfs::{
	ops::filehandle,
	types::{GETFH4res, GETFH4resok},
};

use self::{
	rpc::{Auth, AuthStat, Message, MessageBody, ReplyAcceptedStat, ReplyBody, ReplyRejected},
	state::{Node, State},
	types::{nfs_argop4, nfs_resop4, nfsstat4, NFS_PROG, NFS_VERS, RPC_VERS},
	types::{nfs_fh4, COMPOUND4res},
	xdr::{Decoder, Encoder, Error},
};
use std::path::Path;
use std::sync::Arc;
use tangram_client as tg;
use tg::{Client, WrapErr};
use tokio::{
	net::{TcpListener, TcpStream},
	sync::RwLock,
};

mod ops;
mod rpc;
mod state;
mod types;
mod xdr;

const ROOT: nfs_fh4 = nfs_fh4(0);

#[derive(Clone)]
pub struct Server {
	client: Arc<dyn Client>,
	state: Arc<RwLock<State>>,
}

#[derive(Debug, Clone)]
pub struct Context {
	#[allow(dead_code)]
	minor_version: u32,
	current_file_handle: Option<nfs_fh4>,
	saved_file_handle: Option<nfs_fh4>,
}

impl Server {
	pub fn new(client: &dyn Client) -> Self {
		let client = Arc::from(client.clone_box());
		let state = Arc::new(RwLock::new(State::default()));
		Self { client, state }
	}

	pub async fn serve(&self, port: u16) -> crate::Result<()> {
		let listener = TcpListener::bind(format!("localhost:{port}"))
			.await
			.wrap_err("Failed to bind the server.")?;
		loop {
			let (conn, addr) = listener
				.accept()
				.await
				.wrap_err("Failed to accept the connection.")?;
			tracing::info!(?addr, "Accepted client connection.");
			let server = self.clone();
			tokio::task::spawn(async move {
				if let Err(error) = server.handle_connection(conn).await {
					match error {
						Error::Io(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
							tracing::info!(?addr, "The connection was closed.");
						},
						error => tracing::error!(?error),
					}
				}
			});
		}
	}

	async fn handle_connection(&self, mut stream: TcpStream) -> Result<(), Error> {
		loop {
			let fragments = rpc::read_fragments(&mut stream).await?;
			let mut decoder = Decoder::from_bytes(&fragments);
			let mut buffer = Vec::new();
			while let Ok(message) = decoder.decode::<rpc::Message>() {
				let xid = message.xid;
				let Some(body) = self.handle_message(message, &mut decoder).await else {
					continue;
				};
				buffer.clear();
				let mut encoder = Encoder::new(&mut buffer);
				let reply = rpc::Message {
					xid,
					body: MessageBody::Reply(body),
				};
				encoder.encode(&reply)?;
				rpc::write_fragments(&mut stream, &buffer).await?;
			}
		}
	}

	#[tracing::instrument(skip(self, decoder), ret)]
	async fn handle_message(
		&self,
		message: Message,
		decoder: &mut Decoder<'_>,
	) -> Option<ReplyBody> {
		match message.clone().body {
			MessageBody::Call(call) => {
				if call.rpcvers != RPC_VERS {
					tracing::error!(?call, "Version mismatch.");
					let rejected = ReplyRejected::RpcMismatch {
						low: RPC_VERS,
						high: RPC_VERS,
					};
					let body = ReplyBody::Rejected(rejected);
					return Some(body);
				}

				if call.vers != NFS_VERS {
					tracing::error!(?call, "Program mismatch.");
					return Some(rpc::error(
						None,
						ReplyAcceptedStat::ProgramMismatch {
							low: NFS_VERS,
							high: NFS_VERS,
						},
					));
				}

				if call.prog != NFS_PROG {
					tracing::error!(?call, "Expected NFS4_PROGRAM but got {}.", call.prog);
					return Some(rpc::error(None, ReplyAcceptedStat::ProgramUnavailable));
				}

				let reply = match call.proc {
					0 => self.handle_null(),
					1 => self.handle_compound(call.cred, call.verf, decoder).await,
					_ => rpc::error(None, ReplyAcceptedStat::ProcedureUnavailable),
				};

				Some(reply)
			},
			MessageBody::Reply(reply) => {
				tracing::warn!(?reply, "Ignoring reply");
				None
			},
		}
	}

	// Check if credential and verification are valid.
	async fn handle_auth(&self, _cred: Auth, _verf: Auth) -> Result<Option<Auth>, AuthStat> {
		Ok(None)
	}

	#[tracing::instrument(skip(self))]
	fn handle_null(&self) -> ReplyBody {
		rpc::success(None, ())
	}

	// See <https://datatracker.ietf.org/doc/html/rfc7530#section-17.2>.
	async fn handle_compound(
		&self,
		cred: Auth,
		verf: Auth,
		decoder: &mut Decoder<'_>,
	) -> ReplyBody {
		// Deserialize the arguments up front.
		let args = match decoder.decode::<types::COMPOUND4args>() {
			Ok(args) => args,
			Err(e) => {
				tracing::error!(?e, "Failed to decode COMPOUND args.");
				return rpc::error(None, ReplyAcceptedStat::GarbageArgs);
			},
		};

		// Handle verification.
		let verf = match self.handle_auth(cred, verf).await {
			Ok(verf) => verf,
			Err(stat) => return rpc::reject(ReplyRejected::AuthError(stat)),
		};

		// Handle the operations.
		let types::COMPOUND4args {
			tag,
			minorversion,
			argarray,
			..
		} = args;

		// Create the context
		let mut ctx = Context {
			minor_version: minorversion,
			current_file_handle: None,
			saved_file_handle: None,
		};

		let mut resarray = Vec::new(); // Result buffer.
		let mut status = nfsstat4::NFS4_OK; // Most recent status code.
		for arg in argarray {
			tracing::info!(?arg);
			let result = match arg {
				nfs_argop4::OP_ILLEGAL => nfs_resop4::OP_ILLEGAL(types::ILLEGAL4res {
					status: nfsstat4::NFS4ERR_OP_ILLEGAL,
				}),
				nfs_argop4::OP_ACCESS(arg) => {
					nfs_resop4::OP_ACCESS(self.handle_access(&ctx, arg).await)
				},
				nfs_argop4::OP_CLOSE(arg) => {
					nfs_resop4::OP_CLOSE(self.handle_close(&ctx, arg).await)
				},
				nfs_argop4::OP_GETATTR(arg) => {
					nfs_resop4::OP_GETATTR(self.handle_getattr(&ctx, arg).await)
				},
				nfs_argop4::OP_GETFH => {
					let res = match filehandle::get(&ctx) {
						Ok(object) => GETFH4res::NFS4_OK(GETFH4resok { object }),
						Err(error) => GETFH4res::Error(error),
					};
					nfs_resop4::OP_GETFH(res)
				},
				nfs_argop4::OP_LOOKUP(arg) => {
					nfs_resop4::OP_LOOKUP(self.handle_lookup(&mut ctx, arg).await)
				},
				nfs_argop4::OP_OPEN(arg) => {
					nfs_resop4::OP_OPEN(self.handle_open(&mut ctx, arg).await)
				},
				nfs_argop4::OP_PUTFH(arg) => {
					filehandle::put(&mut ctx, arg.object);
					nfs_resop4::OP_PUTFH(types::PUTFH4res {
						status: nfsstat4::NFS4_OK,
					})
				},
				nfs_argop4::OP_PUTROOTFH => {
					filehandle::put(&mut ctx, ROOT);
					nfs_resop4::OP_PUTROOTFH(types::PUTROOTFH4res {
						status: nfsstat4::NFS4_OK,
					})
				},
				nfs_argop4::OP_READ(arg) => nfs_resop4::OP_READ(self.handle_read(&ctx, arg).await),
				nfs_argop4::OP_READDIR(arg) => {
					nfs_resop4::OP_READDIR(self.handle_readdir(&ctx, arg).await)
				},
				nfs_argop4::OP_READLINK => {
					nfs_resop4::OP_READLINK(self.handle_readlink(&ctx).await)
				},
				nfs_argop4::OP_RENEW(arg) => nfs_resop4::OP_RENEW(self.handle_renew(arg)),
				nfs_argop4::OP_RESTOREFH => {
					filehandle::restore(&mut ctx);
					nfs_resop4::OP_RESTOREFH(types::RESTOREFH4res {
						status: nfsstat4::NFS4_OK,
					})
				},
				nfs_argop4::OP_SAVEFH => {
					filehandle::save(&mut ctx);
					nfs_resop4::OP_SAVEFH(types::SAVEFH4res {
						status: nfsstat4::NFS4_OK,
					})
				},
				nfs_argop4::OP_SECINFO(arg) => {
					nfs_resop4::OP_SECINFO(self.handle_sec_info(&ctx, arg).await)
				},
				nfs_argop4::OP_SETCLIENTID(arg) => {
					nfs_resop4::OP_SETCLIENTID(self.handle_set_client_id(arg).await)
				},
				nfs_argop4::OP_SETCLIENTID_CONFIRM(arg) => {
					nfs_resop4::OP_SETCLIENTID_CONFIRM(self.handle_set_client_id_confirm(arg).await)
				},
				nfs_argop4::Unimplemented(arg) => types::nfs_resop4::Unknown(arg),
			};

			status = result.status();
			resarray.push(result.clone());
			if status != nfsstat4::NFS4_OK {
				tracing::error!(?status, ?result, "Method failed.");
				break;
			}
		}

		let results = COMPOUND4res {
			status,
			tag,
			resarray,
		};
		rpc::success(verf, results)
	}

	pub async fn get_node(&self, node: nfs_fh4) -> Option<Arc<Node>> {
		self.state.read().await.nodes.get(&node.0).cloned()
	}
}

pub async fn mount(mountpoint: &Path, port: u16) -> crate::Result<()> {
	let _ = tokio::process::Command::new("dns-sd")
		.args([
			"-P",
			"Tangram",
			"_nfs._tcp",
			"local",
			&port.to_string(),
			"Tangram",
			"::1",
			"path=/",
		])
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()
		.wrap_err("Failed to spawn dns-sd.")?;
	let _ = tokio::process::Command::new("umount")
		.arg("-f")
		.arg(mountpoint)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.await
		.wrap_err("Failed to unmount.")?;
	tokio::process::Command::new("mount_nfs")
		.arg("-o")
		.arg(format!("tcp,vers=4.0,port={port}"))
		.arg("Tangram:/")
		.arg(mountpoint)
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.status()
		.await
		.wrap_err("Failed to mount.")?
		.success()
		.then_some(())
		.wrap_err("Failed to mount NFS share.")?;
	Ok(())
}
