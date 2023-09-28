use super::{
	compound::{Arg, CompoundArgs, CompoundReply, ResultOp},
	ops::*,
	rpc::{
		self, Auth, AuthStat, Message, MessageBody, ReplyAcceptedStat, ReplyBody, ReplyRejected,
	},
	state::{Node, State},
	types::*,
	xdr::{Decoder, Encoder, Error},
};
use crate::Client;
use std::sync::Arc;
use tokio::{
	net::{TcpListener, TcpStream},
	sync::RwLock,
};

#[derive(Clone)]
pub struct Server {
	pub client: Client,
	pub state: Arc<RwLock<State>>,
}

#[derive(Debug, Clone)]
pub struct Context {
	pub minor_version: u32,
	pub current_file_handle: Option<FileHandle>,
	pub saved_file_handle: Option<FileHandle>,
}

impl Server {
	pub fn new(client: Client) -> Self {
		Self {
			client,
			state: Arc::new(RwLock::new(State::default())),
		}
	}

	/// Serve NFS4 requests on [port].
	pub async fn serve(&self, port: u16) -> crate::Result<()> {
		let listener = TcpListener::bind(format!("localhost:{port}")).await?;
		tracing::info!("🚀 Serving NFS requests on {port}.");
		loop {
			let (conn, addr) = listener.accept().await?;
			tracing::info!(?addr, "Accepted client connection.");
			let server = self.clone();
			tokio::task::spawn(async move {
				if let Err(e) = server.handle_connection(conn).await {
					match e {
						Error::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
							tracing::info!(?addr, "Closing connection");
						},
						e => tracing::error!(?e),
					}
				}
			});
		}
	}

	/// Handle an incoming TCP stream connection.
	async fn handle_connection(&self, mut stream: TcpStream) -> Result<(), Error> {
		loop {
			let fragments = rpc::read_fragments(&mut stream).await?;
			let mut decoder = Decoder::from_bytes(&fragments);
			let mut reply_buf = Vec::new();

			while let Ok(message) = decoder.decode::<rpc::Message>() {
				let xid = message.xid;
				if let Some(body) = self.handle_message(message, &mut decoder).await {
					reply_buf.clear();
					let mut encoder = Encoder::from_writer(&mut reply_buf);
					let reply = rpc::Message {
						xid,
						body: MessageBody::Reply(body),
					};
					encoder.encode(&reply)?;
					rpc::write_fragments(&mut stream, &reply_buf).await?;
				}
			}
		}
	}

	/// Handle a single message pulled off the TCP stream.
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

	// See https://datatracker.ietf.org/doc/html/rfc7530#section-17.2 for description.
	// - An incoming COMPOUND procedure contains a variable length array of args marked by their opcode.
	// - The server is allowed to deserialize all arguments up front, or one at a time. For convenience we deserialize up front.
	// - The return of this is an array of as many operations as the server could successfully complete.
	// - The status field of the reply is the
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
				Arg::SetCLientIdConfirm(arg) => {
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

const ROOT: FileHandle = FileHandle { node: 0 };
