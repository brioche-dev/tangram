use self::{
	compound::{Arg, CompoundArgs, CompoundReply, ResultOp},
	ops::*,
	rpc::{Auth, AuthStat, Message, MessageBody, ReplyAcceptedStat, ReplyBody, ReplyRejected},
	state::{Node, State},
	types::*,
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

mod compound;
mod ops;
mod rpc;
mod state;
mod types;
mod xdr;

const ROOT: FileHandle = FileHandle { node: 0 };

#[derive(Clone)]
pub struct Server {
	client: Arc<dyn Client>,
	state: Arc<RwLock<State>>,
}

#[derive(Debug, Clone)]
pub struct Context {
	#[allow(dead_code)]
	minor_version: u32,
	current_file_handle: Option<FileHandle>,
	saved_file_handle: Option<FileHandle>,
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
		let args = match decoder.decode::<CompoundArgs>() {
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
		let CompoundArgs {
			tag,
			minor_version,
			args,
			..
		} = args;

		// Create the context
		let mut ctx = Context {
			minor_version,
			current_file_handle: None,
			saved_file_handle: None,
		};

		// Debugging
		let opcodes = args.iter().map(|arg| arg.opcode()).collect::<Vec<_>>();
		tracing::info!(?tag, ?minor_version, ?opcodes, "COMPOUND");

		let mut results = Vec::new(); // Result buffer.
		let mut status = NFS4_OK; // Most recent status code.
		for arg in args.into_iter() {
			tracing::info!(?arg);
			let result = match arg {
				Arg::Illegal => ResultOp::Illegal,
				Arg::Access(arg) => ResultOp::Access(self.handle_access(&ctx, arg).await),
				Arg::Close(arg) => ResultOp::Close(self.handle_close(&ctx, arg).await),
				Arg::GetAttr(arg) => ResultOp::GetAttr(self.handle_getattr(&ctx, arg).await),
				Arg::GetFileHandle => ResultOp::GetFileHandle(filehandle::get(&ctx)),
				Arg::Lookup(arg) => ResultOp::LookupResult(self.handle_lookup(&mut ctx, arg).await),
				Arg::Open(arg) => ResultOp::OpenResult(self.handle_open(&mut ctx, arg).await),
				Arg::PutFileHandle(fh) => {
					filehandle::put(&mut ctx, fh);
					ResultOp::PutFileHandle(NFS4_OK)
				},
				Arg::PutRootFileHandle => {
					filehandle::put(&mut ctx, ROOT);
					ResultOp::PutRootFileHandle(NFS4_OK)
				},
				Arg::Read(arg) => ResultOp::Read(self.handle_read(&ctx, arg).await),
				Arg::ReadDir(arg) => ResultOp::ReadDir(self.handle_readdir(&ctx, arg).await),
				Arg::ReadLink => ResultOp::ReadLink(self.handle_readlink(&ctx).await),
				Arg::Renew(client) => ResultOp::Renew(self.handle_renew(client)),
				Arg::RestoreFileHandle => {
					filehandle::restore(&mut ctx);
					ResultOp::RestoreFileHandle(0)
				},
				Arg::SaveFileHandle => {
					filehandle::save(&mut ctx);
					ResultOp::SaveFileHandle(NFS4_OK)
				},
				Arg::SecInfo(arg) => ResultOp::SecInfo(self.handle_sec_info(&ctx, &arg).await),
				Arg::SetClientId(arg) => {
					ResultOp::SetClientId(self.handle_set_client_id(arg).await)
				},
				Arg::SetClientIdConfirm(arg) => {
					ResultOp::SetClientIdConfirm(self.handle_set_client_id_confirm(arg).await)
				},
				arg => ResultOp::Unsupported(arg.opcode()),
			};

			status = result.status();
			results.push(result);
			if status != NFS4_OK {
				break;
			}
		}

		let results = CompoundReply {
			status,
			tag,
			results,
		};

		rpc::success(verf, results)
	}

	pub async fn get_node(&self, node: u64) -> Option<Arc<Node>> {
		self.state.read().await.nodes.get(&node).cloned()
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
