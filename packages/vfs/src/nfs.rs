use self::{
	rpc::{Auth, AuthStat, Message, MessageBody, ReplyAcceptedStat, ReplyBody, ReplyRejected},
	types::{
		bitmap4, cb_client4, change_info4, dirlist4, entry4, fattr4, fs_locations4, fsid4, locker4,
		nfs_argop4, nfs_fh4, nfs_ftype4, nfs_resop4, nfsace4, nfsstat4, nfstime4, open_claim4,
		open_delegation4, open_delegation_type4, pathname4, specdata4, stateid4, verifier4,
		ACCESS4args, ACCESS4res, ACCESS4resok, CLOSE4args, CLOSE4res, COMPOUND4res, GETATTR4args,
		GETATTR4res, GETATTR4resok, GETFH4res, GETFH4resok, LOCK4args, LOCK4res, LOCK4resok,
		LOCKU4args, LOCKU4res, LOOKUP4args, LOOKUP4res, OPEN4args, OPEN4res, OPEN4resok,
		PUTFH4args, PUTFH4res, READ4args, READ4res, READ4resok, READDIR4args, READDIR4res,
		READDIR4resok, READLINK4res, READLINK4resok, RELEASE_LOCKOWNER4args, RELEASE_LOCKOWNER4res,
		RENEW4args, RENEW4res, RESTOREFH4res, SAVEFH4res, SECINFO4args, SECINFO4res,
		SETCLIENTID4args, SETCLIENTID4res, SETCLIENTID4resok, SETCLIENTID_CONFIRM4args,
		SETCLIENTID_CONFIRM4res, ACCESS4_EXECUTE, ACCESS4_LOOKUP, ACCESS4_READ, ANONYMOUS_STATE_ID,
		FATTR4_ACL, FATTR4_ACLSUPPORT, FATTR4_ARCHIVE, FATTR4_CANSETTIME, FATTR4_CASE_INSENSITIVE,
		FATTR4_CASE_PRESERVING, FATTR4_CHANGE, FATTR4_CHOWN_RESTRICTED, FATTR4_FH_EXPIRE_TYPE,
		FATTR4_FILEHANDLE, FATTR4_FILEID, FATTR4_FILES_AVAIL, FATTR4_FILES_FREE,
		FATTR4_FILES_TOTAL, FATTR4_FSID, FATTR4_FS_LOCATIONS, FATTR4_HIDDEN, FATTR4_HOMOGENEOUS,
		FATTR4_LEASE_TIME, FATTR4_LINK_SUPPORT, FATTR4_MAXFILESIZE, FATTR4_MAXLINK, FATTR4_MAXNAME,
		FATTR4_MAXREAD, FATTR4_MAXWRITE, FATTR4_MIMETYPE, FATTR4_MODE, FATTR4_MOUNTED_ON_FILEID,
		FATTR4_NAMED_ATTR, FATTR4_NO_TRUNC, FATTR4_NUMLINKS, FATTR4_OWNER, FATTR4_OWNER_GROUP,
		FATTR4_QUOTA_AVAIL_HARD, FATTR4_QUOTA_AVAIL_SOFT, FATTR4_QUOTA_USED, FATTR4_RAWDEV,
		FATTR4_RDATTR_ERROR, FATTR4_SIZE, FATTR4_SPACE_AVAIL, FATTR4_SPACE_FREE,
		FATTR4_SPACE_TOTAL, FATTR4_SPACE_USED, FATTR4_SUPPORTED_ATTRS, FATTR4_SYMLINK_SUPPORT,
		FATTR4_SYSTEM, FATTR4_TIME_ACCESS, FATTR4_TIME_BACKUP, FATTR4_TIME_CREATE,
		FATTR4_TIME_DELTA, FATTR4_TIME_METADATA, FATTR4_TIME_MODIFY, FATTR4_TYPE,
		FATTR4_UNIQUE_HANDLES, MODE4_RGRP, MODE4_ROTH, MODE4_RUSR, MODE4_XGRP, MODE4_XOTH,
		MODE4_XUSR, NFS4_OTHER_SIZE, NFS_PROG, NFS_VERS, READ_BYPASS_STATE_ID, RPC_VERS,
	},
	xdr::{Decoder, Encoder, Error},
};
use num::ToPrimitive;
use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	sync::{Arc, Weak},
};
use tangram_client as tg;
use tangram_error::{Result, WrapErr};
use tokio::{
	io::{AsyncReadExt, AsyncSeekExt},
	net::{TcpListener, TcpStream},
};

mod rpc;
mod types;
mod xdr;

const ROOT: nfs_fh4 = nfs_fh4(0);

#[derive(Clone)]
pub struct Server {
	inner: Arc<Inner>,
}

struct Inner {
	client: Box<dyn tg::Client>,
	path: PathBuf,
	state: tokio::sync::RwLock<State>,
	task: Task,
}

type Task = (
	std::sync::Mutex<Option<tokio::task::JoinHandle<Result<()>>>>,
	std::sync::Mutex<Option<tokio::task::AbortHandle>>,
);

#[derive(Clone)]
struct State {
	nodes: BTreeMap<u64, Arc<Node>>,
	readers: BTreeMap<u64, Arc<tokio::sync::RwLock<tg::blob::Reader>>>,
	clients: HashMap<Vec<u8>, ClientData>,
	index: u64,
}

#[derive(Debug)]
struct Node {
	id: u64,
	parent: Weak<Self>,
	kind: NodeKind,
}

/// An node's kind.
#[derive(Debug)]
enum NodeKind {
	Root {
		children: tokio::sync::RwLock<BTreeMap<String, Arc<Node>>>,
	},
	Directory {
		directory: tg::Directory,
		children: tokio::sync::RwLock<BTreeMap<String, Arc<Node>>>,
	},
	File {
		file: tg::File,
		size: u64,
	},
	Symlink {
		symlink: tg::Symlink,
	},
}

#[derive(Clone, Debug)]
pub struct ClientData {
	pub server_id: u64,
	pub client_verifier: verifier4,
	pub server_verifier: verifier4,
	pub callback: cb_client4,
	pub callback_ident: u32,
	pub confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct Context {
	#[allow(dead_code)]
	minor_version: u32,
	current_file_handle: Option<nfs_fh4>,
	saved_file_handle: Option<nfs_fh4>,
}

impl Server {
	pub async fn start(client: &dyn tg::Client, path: &Path, port: u16) -> Result<Self> {
		// Create the server.
		let client = client.clone_box();
		let root = Arc::new_cyclic(|root| Node {
			id: 0,
			parent: root.clone(),
			kind: NodeKind::Root {
				children: tokio::sync::RwLock::new(BTreeMap::default()),
			},
		});
		let nodes = [(0, root)].into_iter().collect();
		let state = tokio::sync::RwLock::new(State {
			nodes,
			readers: BTreeMap::default(),
			clients: HashMap::new(),
			index: 0,
		});
		let task = (std::sync::Mutex::new(None), std::sync::Mutex::new(None));
		let server = Self {
			inner: Arc::new(Inner {
				client,
				path: path.to_owned(),
				state,
				task,
			}),
		};

		// Spawn the task.
		let task = tokio::spawn({
			let server = server.clone();
			async move { server.serve(port).await }
		});
		let abort = task.abort_handle();
		server.inner.task.1.lock().unwrap().replace(abort);
		server.inner.task.0.lock().unwrap().replace(task);

		// Mount.
		Self::mount(path, port).await?;

		Ok(server)
	}

	async fn mount(path: &Path, port: u16) -> crate::Result<()> {
		Self::unmount(path).await?;

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

		tokio::process::Command::new("mount_nfs")
			.arg("-o")
			.arg(format!("tcp,vers=4.0,port={port}"))
			.arg("Tangram:/")
			.arg(path)
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null())
			.status()
			.await
			.wrap_err("Failed to mount.")?
			.success()
			.then_some(())
			.wrap_err("Failed to mount the VFS.")?;

		Ok(())
	}

	async fn unmount(path: &Path) -> Result<()> {
		let _ = tokio::process::Command::new("umount")
			.arg("-f")
			.arg(path)
			.stdout(std::process::Stdio::null())
			.stderr(std::process::Stdio::null())
			.status()
			.await
			.wrap_err("Failed to unmount the VFS.")?;
		Ok(())
	}

	pub fn stop(&self) {
		// Abort the task.
		if let Some(handle) = self.inner.task.1.lock().unwrap().as_ref() {
			handle.abort();
		};
	}

	pub async fn join(&self) -> Result<()> {
		// Join the task.
		let task = self.inner.task.0.lock().unwrap().take();
		if let Some(task) = task {
			match task.await {
				Ok(result) => Ok(result),
				Err(error) if error.is_cancelled() => Ok(Ok(())),
				Err(error) => Err(error),
			}
			.unwrap()?;
		}

		// Unmount.
		Self::unmount(&self.inner.path).await?;

		Ok(())
	}

	async fn serve(&self, port: u16) -> crate::Result<()> {
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
	#[allow(clippy::unused_async, clippy::unnecessary_wraps)]
	async fn handle_auth(&self, _cred: Auth, _verf: Auth) -> Result<Option<Auth>, AuthStat> {
		Ok(None)
	}

	#[tracing::instrument(skip(self))]
	fn handle_null(&self) -> ReplyBody {
		rpc::success(None, ())
	}

	// See <https://datatracker.ietf.org/doc/html/rfc7530#section-17.2>.
	#[allow(clippy::too_many_lines)]
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
				nfs_argop4::OP_GETFH => nfs_resop4::OP_GETFH(Self::handle_get_file_handle(&ctx)),
				nfs_argop4::OP_LOCK(arg) => {
					nfs_resop4::OP_LOCK(self.handle_lock(&mut ctx, arg).await)
				},
				nfs_argop4::OP_LOCKU(arg) => {
					nfs_resop4::OP_LOCKU(self.handle_locku(&mut ctx, arg).await)
				},
				nfs_argop4::OP_LOOKUP(arg) => {
					nfs_resop4::OP_LOOKUP(self.handle_lookup(&mut ctx, arg).await)
				},
				nfs_argop4::OP_OPEN(arg) => {
					nfs_resop4::OP_OPEN(self.handle_open(&mut ctx, arg).await)
				},
				nfs_argop4::OP_PUTFH(arg) => {
					nfs_resop4::OP_PUTFH(Self::handle_put_file_handle(&mut ctx, &arg))
				},
				nfs_argop4::OP_PUTROOTFH => {
					let _ = Self::handle_put_file_handle(&mut ctx, &PUTFH4args { object: ROOT });
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
					nfs_resop4::OP_RESTOREFH(Self::handle_restore_file_handle(&mut ctx))
				},
				nfs_argop4::OP_SAVEFH => {
					nfs_resop4::OP_SAVEFH(Self::handle_save_file_handle(&mut ctx))
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
				nfs_argop4::OP_RELEASE_LOCKOWNER(arg) => nfs_resop4::OP_RELEASE_LOCKOWNER(
					self.handle_release_lockowner(&mut ctx, arg).await,
				),
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

	async fn get_node(&self, node: nfs_fh4) -> Option<Arc<Node>> {
		self.inner.state.read().await.nodes.get(&node.0).cloned()
	}
}

impl Server {
	#[tracing::instrument(skip(self))]
	async fn handle_access(&self, ctx: &Context, arg: ACCESS4args) -> ACCESS4res {
		let Some(fh) = ctx.current_file_handle else {
			return ACCESS4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return ACCESS4res::Error(nfsstat4::NFS4ERR_BADHANDLE);
		};

		let access = match &node.kind {
			NodeKind::Root { .. } | NodeKind::Directory { .. } => {
				ACCESS4_EXECUTE | ACCESS4_READ | ACCESS4_LOOKUP
			},
			NodeKind::Symlink { .. } => ACCESS4_READ,
			NodeKind::File { file, .. } => {
				let is_executable = match file.executable(self.inner.client.as_ref()).await {
					Ok(b) => b,
					Err(e) => {
						tracing::error!(?e, "Failed to lookup executable bit for file.");
						return ACCESS4res::Error(nfsstat4::NFS4ERR_IO);
					},
				};
				if is_executable {
					ACCESS4_EXECUTE | ACCESS4_READ
				} else {
					ACCESS4_READ
				}
			},
		};

		let supported = arg.access & access;
		let resok = ACCESS4resok { supported, access };

		ACCESS4res::NFS4_OK(resok)
	}

	#[tracing::instrument(skip(self))]
	async fn handle_close(&self, ctx: &Context, arg: CLOSE4args) -> CLOSE4res {
		let Some(fh) = ctx.current_file_handle else {
			return CLOSE4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return CLOSE4res::Error(nfsstat4::NFS4ERR_BADHANDLE);
		};

		let mut stateid = arg.open_stateid;

		if let NodeKind::File { .. } = &node.kind {
			let mut state = self.inner.state.write().await;
			let index = u64::from_be_bytes(stateid.other[0..8].try_into().unwrap());
			if state.readers.remove(&index).is_none() {
				return CLOSE4res::Error(nfsstat4::NFS4ERR_BAD_STATEID);
			}
		}

		stateid.seqid = stateid.seqid.increment();

		CLOSE4res::NFS4_OK(stateid)
	}

	#[tracing::instrument(skip(self))]
	async fn handle_getattr(&self, ctx: &Context, arg: GETATTR4args) -> GETATTR4res {
		let Some(fh) = ctx.current_file_handle else {
			tracing::error!("Missing current file handle.");
			return GETATTR4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		match self.get_attr(fh, arg.attr_request).await {
			Ok(obj_attributes) => GETATTR4res::NFS4_OK(GETATTR4resok { obj_attributes }),
			Err(e) => GETATTR4res::Error(e),
		}
	}

	async fn get_attr(&self, file_handle: nfs_fh4, requested: bitmap4) -> Result<fattr4, nfsstat4> {
		if requested.0.is_empty() {
			return Ok(fattr4 {
				attrmask: bitmap4(Vec::default()),
				attr_vals: Vec::new(),
			});
		}

		let Some(data) = self.get_file_attr_data(file_handle).await else {
			tracing::error!(?file_handle, "Missing attr data.");
			return Err(nfsstat4::NFS4ERR_NOENT);
		};

		let attrmask = data.supported_attrs.intersection(&requested);
		let attr_vals = data.to_bytes(&attrmask);

		Ok(fattr4 {
			attrmask,
			attr_vals,
		})
	}

	#[allow(clippy::similar_names)]
	async fn get_file_attr_data(&self, file_handle: nfs_fh4) -> Option<FileAttrData> {
		let state = &self.inner.state.read().await;
		let node = state.nodes.get(&file_handle.0)?;
		let data = match &node.kind {
			NodeKind::Root { .. } => FileAttrData::new(file_handle, nfs_ftype4::NF4DIR, 0, O_RX),
			NodeKind::Directory { children, .. } => {
				let len = children.read().await.len();
				FileAttrData::new(file_handle, nfs_ftype4::NF4DIR, len, O_RX)
			},
			NodeKind::File { file, size } => {
				let is_executable = match file.executable(self.inner.client.as_ref()).await {
					Ok(b) => b,
					Err(e) => {
						tracing::error!(?e, "Failed to lookup executable bit for file.");
						return None;
					},
				};
				let mode = if is_executable { O_RX } else { O_RDONLY };

				FileAttrData::new(
					file_handle,
					nfs_ftype4::NF4REG,
					size.to_usize().unwrap(),
					mode,
				)
			},
			NodeKind::Symlink { .. } => {
				FileAttrData::new(file_handle, nfs_ftype4::NF4LNK, 1, O_RDONLY)
			},
		};
		Some(data)
	}

	#[tracing::instrument(skip(self), ret)]
	async fn handle_lock(&self, ctx: &mut Context, arg: LOCK4args) -> LOCK4res {
		let lock_stateid = match arg.locker {
			locker4::TRUE(open_to_lock_owner) => open_to_lock_owner.open_stateid,
			locker4::FALSE(exist_lock_owner) => exist_lock_owner.lock_stateid,
		};
		let resok = LOCK4resok { lock_stateid };
		LOCK4res::NFS4_OK(resok)
	}

	#[tracing::instrument(skip(self), ret)]
	async fn handle_locku(&self, ctx: &mut Context, arg: LOCKU4args) -> LOCKU4res {
		LOCKU4res::NFS4_OK(arg.lock_stateid)
	}

	#[tracing::instrument(skip(self))]
	async fn handle_lookup(&self, ctx: &mut Context, arg: LOOKUP4args) -> LOOKUP4res {
		let Some(fh) = ctx.current_file_handle else {
			return LOOKUP4res {
				status: nfsstat4::NFS4ERR_NOFILEHANDLE,
			};
		};

		let Ok(name) = std::str::from_utf8(&arg.objname) else {
			return LOOKUP4res {
				status: nfsstat4::NFS4ERR_NOENT,
			};
		};

		match self.lookup(fh, name).await {
			Ok(fh) => {
				ctx.current_file_handle = Some(fh);
				LOOKUP4res {
					status: nfsstat4::NFS4_OK,
				}
			},
			Err(status) => LOOKUP4res { status },
		}
	}

	async fn lookup(&self, parent: nfs_fh4, name: &str) -> Result<nfs_fh4, nfsstat4> {
		let parent_node = self
			.inner
			.state
			.read()
			.await
			.nodes
			.get(&parent.0)
			.cloned()
			.ok_or(nfsstat4::NFS4ERR_NOENT)?;
		let node = self.get_or_create_child_node(parent_node, name).await?;
		let fh = nfs_fh4(node.id);
		Ok(fh)
	}

	async fn get_or_create_child_node(
		&self,
		parent_node: Arc<Node>,
		name: &str,
	) -> Result<Arc<Node>, nfsstat4> {
		if name == "." {
			return Ok(parent_node);
		}

		if name == ".." {
			let parent_parent_node = parent_node.parent.upgrade().ok_or(nfsstat4::NFS4ERR_IO)?;
			return Ok(parent_parent_node);
		}

		match &parent_node.kind {
			NodeKind::Root { children } | NodeKind::Directory { children, .. } => {
				if let Some(child) = children.read().await.get(name).cloned() {
					return Ok(child);
				}
			},
			_ => {
				tracing::error!("Cannot create child on File or Symlink.");
				return Err(nfsstat4::NFS4ERR_NOTDIR);
			},
		}

		let child_artifact = match &parent_node.kind {
			NodeKind::Root { .. } => {
				let id = name.parse().map_err(|e| {
					tracing::error!(?e, ?name, "Failed to parse artifact ID.");
					nfsstat4::NFS4ERR_NOENT
				})?;
				tg::Artifact::with_id(id)
			},

			NodeKind::Directory { directory, .. } => {
				let entries = directory
					.entries(self.inner.client.as_ref())
					.await
					.map_err(|e| {
						tracing::error!(?e, ?name, "Failed to get directory entries.");
						nfsstat4::NFS4ERR_IO
					})?;
				entries.get(name).ok_or(nfsstat4::NFS4ERR_NOENT)?.clone()
			},

			_ => unreachable!(),
		};

		let node_id = self.inner.state.read().await.nodes.len() as u64 + 1000;
		let kind = match child_artifact {
			tg::Artifact::Directory(directory) => {
				let children = tokio::sync::RwLock::new(BTreeMap::default());
				NodeKind::Directory {
					directory,
					children,
				}
			},
			tg::Artifact::File(file) => {
				let contents = file
					.contents(self.inner.client.as_ref())
					.await
					.map_err(|e| {
						tracing::error!(?e, "Failed to get file contents.");
						nfsstat4::NFS4ERR_IO
					})?;
				let size = contents
					.size(self.inner.client.as_ref())
					.await
					.map_err(|e| {
						tracing::error!(?e, "Failed to get size of file's contents.");
						nfsstat4::NFS4ERR_IO
					})?;
				NodeKind::File { file, size }
			},
			tg::Artifact::Symlink(symlink) => NodeKind::Symlink { symlink },
		};
		let child_node = Node {
			id: node_id,
			parent: Arc::downgrade(&parent_node),
			kind,
		};
		let child_node = Arc::new(child_node);

		// Add the child node to the parent node.
		match &parent_node.kind {
			NodeKind::Root { children } | NodeKind::Directory { children, .. } => {
				children
					.write()
					.await
					.insert(name.to_owned(), child_node.clone());
			},
			_ => unreachable!(),
		}

		// Add the child node to the nodes.
		self.inner
			.state
			.write()
			.await
			.nodes
			.insert(child_node.id, child_node.clone());

		Ok(child_node)
	}

	#[tracing::instrument(skip(self))]
	async fn handle_open(&self, ctx: &mut Context, arg: OPEN4args) -> OPEN4res {
		let Some(fh) = ctx.current_file_handle else {
			return OPEN4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let fh = match arg.claim {
			open_claim4::CLAIM_NULL(name) => {
				let Ok(name) = std::str::from_utf8(&name) else {
					return OPEN4res::Error(nfsstat4::NFS4ERR_NOENT);
				};
				match self.lookup(fh, name).await {
					Ok(fh) => fh,
					Err(e) => return OPEN4res::Error(e),
				}
			},
			open_claim4::CLAIM_PREVIOUS(open_delegation_type4::OPEN_DELEGATE_NONE) => fh,
			_ => return OPEN4res::Error(nfsstat4::NFS4ERR_NOTSUPP),
		};

		ctx.current_file_handle = Some(fh);
		let seqid = arg.seqid.increment();

		// Create the stateid.
		let index = {
			let mut state = self.inner.state.write().await;
			let index = state.index;
			state.index += 1;
			index
		};
		let mut other = [0u8; NFS4_OTHER_SIZE];
		other[0..8].copy_from_slice(&index.to_be_bytes());
		let stateid = stateid4 { seqid, other };

		if let NodeKind::File { file, .. } = &self.get_node(fh).await.unwrap().kind {
			let Ok(blob) = file.contents(self.inner.client.as_ref()).await else {
				tracing::error!("Failed to get file's content.");
				return OPEN4res::Error(nfsstat4::NFS4ERR_IO);
			};
			let Ok(reader) = blob.reader(self.inner.client.as_ref()).await else {
				tracing::error!("Failed to create blob reader.");
				return OPEN4res::Error(nfsstat4::NFS4ERR_IO);
			};
			self.inner
				.state
				.write()
				.await
				.readers
				.insert(index, Arc::new(tokio::sync::RwLock::new(reader)));
		}

		let cinfo = change_info4 {
			atomic: false,
			before: 0,
			after: 0,
		};

		let rflags = 0;
		let attrset = bitmap4(vec![]);
		let delegation = open_delegation4::OPEN_DELEGATE_NONE;
		let resok = OPEN4resok {
			stateid,
			cinfo,
			rflags,
			attrset,
			delegation,
		};
		OPEN4res::NFS4_OK(resok)
	}

	#[tracing::instrument(skip(self))]
	async fn handle_read(&self, ctx: &Context, arg: READ4args) -> READ4res {
		let Some(fh) = ctx.current_file_handle else {
			return READ4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};
		let Some(node) = self.get_node(fh).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return READ4res::Error(nfsstat4::NFS4ERR_BADHANDLE);
		};

		// RFC 7530 16.23.4:
		// "If the current file handle is not a regular file, an error will be returned to the client. In the case where the current filehandle represents a directory, NFS4ERR_ISDIR is returned; otherwise, NFS4ERR_INVAL is returned."
		let (file, file_size) = match &node.kind {
			NodeKind::Directory { .. } | NodeKind::Root { .. } => {
				return READ4res::Error(nfsstat4::NFS4ERR_ISDIR)
			},
			NodeKind::Symlink { .. } => return READ4res::Error(nfsstat4::NFS4ERR_INVAL),
			NodeKind::File { file, size, .. } => (file, size),
		};

		// It is allowed for clients to attempt to read past the end of a file, in which case the server returns an empty file.
		if &arg.offset >= file_size {
			return READ4res::NFS4_OK(READ4resok {
				eof: true,
				data: vec![],
			});
		}

		let read_size = arg
			.count
			.to_u64()
			.unwrap()
			.min(file_size - arg.offset)
			.to_usize()
			.unwrap();

		let (data, eof) = if [ANONYMOUS_STATE_ID, READ_BYPASS_STATE_ID].contains(&arg.stateid) {
			// We need to create a reader just for this request.
			let Ok(blob) = file.contents(self.inner.client.as_ref()).await else {
				tracing::error!("Failed to get file's content.");
				return READ4res::Error(nfsstat4::NFS4ERR_IO);
			};
			let Ok(mut reader) = blob.reader(self.inner.client.as_ref()).await else {
				tracing::error!("Failed to create blob reader.");
				return READ4res::Error(nfsstat4::NFS4ERR_IO);
			};
			if let Err(e) = reader.seek(std::io::SeekFrom::Start(arg.offset)).await {
				tracing::error!(?e, "Failed to seek.");
				return READ4res::Error(e.into());
			}
			let mut data = vec![0u8; read_size];
			if let Err(e) = reader.read_exact(&mut data).await {
				tracing::error!(?e, "Failed to read from file.");
				return READ4res::Error(e.into());
			}
			let eof = (arg.offset + arg.count.to_u64().unwrap()) >= *file_size;
			(data, eof)
		} else {
			let index = u64::from_be_bytes(arg.stateid.other[0..8].try_into().unwrap());
			let state = self.inner.state.read().await;
			let Some(reader) = state.readers.get(&index).cloned() else {
				tracing::error!(?arg.stateid, "No reader is registered for the given id.");
				return READ4res::Error(nfsstat4::NFS4ERR_BAD_STATEID);
			};
			let mut reader = reader.write().await;
			if let Err(e) = reader.seek(std::io::SeekFrom::Start(arg.offset)).await {
				tracing::error!(?e, "Failed to seek.");
				return READ4res::Error(e.into());
			}
			let mut data = vec![0u8; read_size];
			if let Err(e) = reader.read_exact(&mut data).await {
				tracing::error!(?e, "Failed to read from file.");
				return READ4res::Error(e.into());
			}
			let eof = (arg.offset + arg.count.to_u64().unwrap()) >= *file_size;
			(data, eof)
		};
		READ4res::NFS4_OK(READ4resok { eof, data })
	}

	#[tracing::instrument(skip(self))]
	async fn handle_readdir(&self, ctx: &Context, arg: READDIR4args) -> READDIR4res {
		let Some(fh) = ctx.current_file_handle else {
			return READDIR4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh).await else {
			return READDIR4res::Error(nfsstat4::NFS4ERR_BADHANDLE);
		};

		let cookie = arg.cookie.to_usize().unwrap();
		let mut count = 0;

		let entries = match &node.kind {
			NodeKind::Directory { directory, .. } => {
				let Ok(entries) = directory.entries(self.inner.client.as_ref()).await else {
					return READDIR4res::Error(nfsstat4::NFS4ERR_IO);
				};
				entries.clone()
			},
			NodeKind::Root { .. } => BTreeMap::default(),
			_ => return READDIR4res::Error(nfsstat4::NFS4ERR_NOTDIR),
		};

		let mut reply = Vec::with_capacity(entries.len());
		let names = entries.keys().map(AsRef::as_ref);

		let mut eof = true;
		for (cookie, name) in [".", ".."]
			.into_iter()
			.chain(names)
			.enumerate()
			.skip(cookie)
		{
			let node = match name {
				"." => node.clone(),
				".." => node.parent.upgrade().unwrap(),
				_ => match self.get_or_create_child_node(node.clone(), name).await {
					Ok(node) => node,
					Err(e) => return READDIR4res::Error(e),
				},
			};
			let attrs = self
				.get_attr(nfs_fh4(node.id), arg.attr_request.clone())
				.await
				.unwrap();
			let cookie = cookie.to_u64().unwrap();
			let name = name.to_owned();

			// Size of the cookie + size of the attr + size of the name
			count += std::mem::size_of_val(&cookie); // u64
			count += 4 + 4 * attrs.attrmask.0.len(); // bitmap4
			count += 4 + attrs.attr_vals.len(); // opaque<>
			count += 4 + name.len(); // utf8_cstr

			if count > arg.dircount.to_usize().unwrap() {
				eof = false;
				break;
			}

			let name = name.as_bytes().into();
			let entry = entry4 {
				cookie,
				name,
				attrs,
			};
			reply.push(entry);
		}

		let cookieverf = fh.0.to_be_bytes();
		let reply = dirlist4 {
			entries: reply,
			eof,
		};
		READDIR4res::NFS4_OK(READDIR4resok { cookieverf, reply })
	}

	#[tracing::instrument(skip(self))]
	async fn handle_readlink(&self, ctx: &Context) -> READLINK4res {
		let Some(fh) = ctx.current_file_handle else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};
		let Some(node) = self.get_node(fh).await else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_NOENT);
		};
		let NodeKind::Symlink { symlink } = &node.kind else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_INVAL);
		};
		let Ok(target) = symlink.target(self.inner.client.as_ref()).await else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_IO);
		};
		let mut response = String::new();
		for component in target.components() {
			match component {
				tg::template::Component::String(string) => {
					response.push_str(string);
				},
				tg::template::Component::Artifact(artifact) => {
					let Ok(id) = artifact.id(self.inner.client.as_ref()).await else {
						return READLINK4res::Error(nfsstat4::NFS4ERR_IO);
					};
					for _ in 0..node.depth() {
						response.push_str("../");
					}
					response.push_str(&id.to_string());
				},
			}
		}

		READLINK4res::NFS4_OK(READLINK4resok {
			link: response.into_bytes(),
		})
	}

	#[tracing::instrument(skip(self))]
	fn handle_renew(&self, arg: RENEW4args) -> RENEW4res {
		RENEW4res {
			status: nfsstat4::NFS4_OK,
		}
	}

	#[tracing::instrument(skip(self))]
	async fn handle_sec_info(&self, ctx: &Context, arg: SECINFO4args) -> SECINFO4res {
		let Some(parent) = ctx.current_file_handle else {
			return SECINFO4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};
		let Ok(name) = std::str::from_utf8(&arg.name) else {
			return SECINFO4res::Error(nfsstat4::NFS4ERR_NOENT);
		};
		match self.lookup(parent, name).await {
			Ok(_) => SECINFO4res::NFS4_OK(vec![]),
			Err(e) => SECINFO4res::Error(e),
		}
	}

	#[tracing::instrument(skip(self))]
	async fn handle_set_client_id(&self, arg: SETCLIENTID4args) -> SETCLIENTID4res {
		let mut state = self.inner.state.write().await;
		let Some(client) = state.clients.get(&arg.client.id) else {
			let (server_id, server_verifier) = state.create_client_data();
			let record = ClientData {
				server_id,
				client_verifier: arg.client.verifier,
				server_verifier,
				callback: arg.callback,
				callback_ident: arg.callback_ident,
				confirmed: false,
			};
			let _ = state.clients.insert(arg.client.id, record);
			return SETCLIENTID4res::NFS4_OK(SETCLIENTID4resok {
				clientid: server_id,
				setclientid_confirm: server_verifier,
			});
		};

		let conditions = [
			client.confirmed,
			client.client_verifier == arg.client.verifier,
			client.callback == arg.callback,
			client.callback_ident == arg.callback_ident,
		];

		if conditions.into_iter().all(|c| c) {
			let clientid = client.server_id;
			let setclientid_confirm = client.server_verifier;
			SETCLIENTID4res::NFS4_OK(SETCLIENTID4resok {
				clientid,
				setclientid_confirm,
			})
		} else {
			SETCLIENTID4res::Error(nfsstat4::NFS4ERR_IO)
		}
	}

	#[tracing::instrument(skip(self))]
	async fn handle_set_client_id_confirm(
		&self,
		arg: SETCLIENTID_CONFIRM4args,
	) -> SETCLIENTID_CONFIRM4res {
		let mut state = self.inner.state.write().await;
		for client in state.clients.values_mut() {
			if client.server_id == arg.clientid {
				if client.server_verifier == arg.setclientid_confirm {
					client.confirmed = true;
					return SETCLIENTID_CONFIRM4res {
						status: nfsstat4::NFS4_OK,
					};
				}
				return SETCLIENTID_CONFIRM4res {
					status: nfsstat4::NFS4ERR_CLID_INUSE,
				};
			}
		}
		SETCLIENTID_CONFIRM4res {
			status: nfsstat4::NFS4ERR_STALE_CLIENTID,
		}
	}

	#[tracing::instrument(skip(self), ret)]
	async fn handle_release_lockowner(
		&self,
		context: &mut Context,
		arg: RELEASE_LOCKOWNER4args,
	) -> RELEASE_LOCKOWNER4res {
		RELEASE_LOCKOWNER4res {
			status: nfsstat4::NFS4_OK,
		}
	}

	fn handle_put_file_handle(ctx: &mut Context, arg: &PUTFH4args) -> PUTFH4res {
		ctx.current_file_handle = Some(arg.object);
		PUTFH4res {
			status: nfsstat4::NFS4_OK,
		}
	}

	fn handle_get_file_handle(ctx: &Context) -> GETFH4res {
		if let Some(object) = ctx.current_file_handle {
			GETFH4res::NFS4_OK(GETFH4resok { object })
		} else {
			GETFH4res::Error(nfsstat4::NFS4ERR_BADHANDLE)
		}
	}

	fn handle_save_file_handle(ctx: &mut Context) -> SAVEFH4res {
		ctx.saved_file_handle = ctx.current_file_handle;
		SAVEFH4res {
			status: nfsstat4::NFS4_OK,
		}
	}

	fn handle_restore_file_handle(ctx: &mut Context) -> RESTOREFH4res {
		ctx.current_file_handle = ctx.saved_file_handle.take();
		RESTOREFH4res {
			status: nfsstat4::NFS4_OK,
		}
	}
}

impl State {
	fn create_client_data(&self) -> (u64, verifier4) {
		let new_id = (self.clients.len() + 1000).to_u64().unwrap();
		(new_id, new_id.to_be_bytes())
	}
}

impl Node {
	fn depth(self: &Arc<Self>) -> usize {
		if self.id == 0 {
			0
		} else {
			1 + self.parent.upgrade().unwrap().depth()
		}
	}
}

pub const O_RDONLY: u32 = MODE4_RUSR | MODE4_RGRP | MODE4_ROTH;
pub const O_RX: u32 = MODE4_XUSR | MODE4_XGRP | MODE4_XOTH | O_RDONLY;

pub const ALL_SUPPORTED_ATTRS: &[u32] = &[
	FATTR4_SUPPORTED_ATTRS,
	FATTR4_TYPE,
	FATTR4_FH_EXPIRE_TYPE,
	FATTR4_CHANGE,
	FATTR4_SIZE,
	FATTR4_LINK_SUPPORT,
	FATTR4_SYMLINK_SUPPORT,
	FATTR4_NAMED_ATTR,
	FATTR4_FSID,
	FATTR4_UNIQUE_HANDLES,
	FATTR4_LEASE_TIME,
	FATTR4_RDATTR_ERROR,
	FATTR4_ARCHIVE,
	FATTR4_CANSETTIME,
	FATTR4_CASE_INSENSITIVE,
	FATTR4_CASE_PRESERVING,
	FATTR4_CHOWN_RESTRICTED,
	FATTR4_FILEHANDLE,
	FATTR4_FILEID,
	FATTR4_FILES_AVAIL,
	FATTR4_FILES_FREE,
	FATTR4_FILES_TOTAL,
	FATTR4_FS_LOCATIONS,
	FATTR4_HIDDEN,
	FATTR4_HOMOGENEOUS,
	FATTR4_MAXFILESIZE,
	FATTR4_MAXLINK,
	FATTR4_MAXNAME,
	FATTR4_MAXREAD,
	FATTR4_MAXWRITE,
	FATTR4_MIMETYPE,
	FATTR4_MODE,
	FATTR4_NO_TRUNC,
	FATTR4_NUMLINKS,
	FATTR4_OWNER,
	FATTR4_OWNER_GROUP,
	FATTR4_QUOTA_AVAIL_HARD,
	FATTR4_QUOTA_AVAIL_SOFT,
	FATTR4_QUOTA_USED,
	FATTR4_RAWDEV,
	FATTR4_SPACE_AVAIL,
	FATTR4_SPACE_FREE,
	FATTR4_SPACE_TOTAL,
	FATTR4_SPACE_USED,
	FATTR4_SYSTEM,
	FATTR4_TIME_ACCESS,
	FATTR4_TIME_BACKUP,
	FATTR4_TIME_CREATE,
	FATTR4_TIME_DELTA,
	FATTR4_TIME_METADATA,
	FATTR4_TIME_MODIFY,
	FATTR4_MOUNTED_ON_FILEID,
];

#[allow(clippy::struct_excessive_bools)]
pub struct FileAttrData {
	supported_attrs: bitmap4,
	file_type: nfs_ftype4,
	/// Defines how file expiry is supposed to be handled. A value of "0" is called FH4_PERSISTENT, which implies the file handle is persistent for the lifetime of the server.
	expire_type: u32,
	/// Defines how file changes happen. Since we don't have changes, we don't care.
	change: u64,
	size: u64,
	/// TRUE if the file system this object is on supports hard links.
	link_support: bool,
	/// TRUE if the file system this object is on supports soft links.
	symlink_support: bool,
	/// Whether this file has any nammed attributes (xattrs). TODO: care about this.
	named_attr: bool,
	/// Identifies which file system the object is on (servers may overlay multiple file systems and report such to the client).
	fsid: fsid4,
	/// TRUE, if two distinct filehandles are guaranteed to refer to two different file system objects.
	unique_handles: bool,
	/// The amount of time this file is valid for, in seconds.
	lease_time: u32,
	/// An error, if we want to return one.
	rdattr_error: i32,
	/// The underlying file handle
	file_handle: nfs_fh4,
	acl: Vec<nfsace4>,
	aclsupport: u32,
	archive: bool,
	cansettime: bool,
	case_insensitive: bool,
	case_preserving: bool,
	chown_restricted: bool,
	fileid: u64,
	files_avail: u64,
	files_free: u64,
	files_total: u64,
	fs_locations: fs_locations4,
	hidden: bool,
	homogeneous: bool,
	maxfilesize: u64,
	maxlink: u32,
	maxname: u32,
	maxread: u64,
	maxwrite: u64,
	mimetype: Vec<String>,
	mode: u32,
	no_trunc: bool,
	numlinks: u32,
	owner: String,
	owner_group: String,
	quota_avail_hard: u64,
	quota_avail_soft: u64,
	quota_used: u64,
	rawdev: specdata4,
	space_avail: u64,
	space_free: u64,
	space_total: u64,
	space_used: u64,
	system: bool,
	time_access: nfstime4,
	time_backup: nfstime4,
	time_create: nfstime4,
	time_delta: nfstime4,
	time_metadata: nfstime4,
	time_modify: nfstime4,
	mounted_on_fileid: u64,
}

impl FileAttrData {
	fn new(file_handle: nfs_fh4, file_type: nfs_ftype4, size: usize, mode: u32) -> FileAttrData {
		let size = size.to_u64().unwrap();
		let mut supported_attrs = bitmap4(Vec::new());
		for attr in ALL_SUPPORTED_ATTRS {
			supported_attrs.set(attr.to_usize().unwrap());
		}
		let change = nfstime4::now().seconds.to_u64().unwrap();
		FileAttrData {
			supported_attrs,
			file_type,
			expire_type: 0,
			change,
			size,
			link_support: true,
			symlink_support: true,
			named_attr: false,
			fsid: fsid4 { major: 0, minor: 1 },
			unique_handles: true,
			lease_time: 1000,
			rdattr_error: 0,
			file_handle,
			acl: Vec::new(),
			aclsupport: 0,
			archive: true,
			cansettime: false,
			case_insensitive: false,
			case_preserving: true,
			chown_restricted: true,
			fileid: file_handle.0,
			files_avail: 0,
			files_free: 0,
			files_total: 1,
			hidden: false,
			homogeneous: true,
			maxfilesize: u64::MAX,
			maxlink: u32::MAX,
			maxname: 512,
			maxread: u64::MAX,
			maxwrite: 0,
			mimetype: Vec::new(),
			mode,
			fs_locations: fs_locations4 {
				fs_root: pathname4(vec!["/".as_bytes().to_owned()]),
				locations: Vec::new(),
			},
			no_trunc: true,
			numlinks: 1,
			owner: "tangram@tangram".to_owned(),
			owner_group: "tangram@tangram".to_owned(),
			quota_avail_hard: 0,
			quota_avail_soft: 0,
			quota_used: 0,
			rawdev: specdata4 {
				specdata1: 0,
				specdata2: 0,
			},
			space_avail: 0,
			space_free: 0,
			space_total: u64::MAX,
			space_used: size.to_u64().unwrap(),
			system: false,
			time_access: nfstime4::new(),
			time_backup: nfstime4::new(),
			time_create: nfstime4::new(),
			time_delta: nfstime4::new(),
			time_metadata: nfstime4::new(),
			time_modify: nfstime4::new(),
			mounted_on_fileid: file_handle.0,
		}
	}

	fn to_bytes(&self, requested: &bitmap4) -> Vec<u8> {
		let mut buf = Vec::with_capacity(256);
		let mut encoder = xdr::Encoder::new(&mut buf);
		for attr in ALL_SUPPORTED_ATTRS.iter().copied() {
			if !requested.get(attr.to_usize().unwrap()) {
				continue;
			}
			match attr {
				FATTR4_SUPPORTED_ATTRS => encoder.encode(&self.supported_attrs.0).unwrap(),
				FATTR4_TYPE => encoder.encode(&self.file_type).unwrap(),
				FATTR4_FH_EXPIRE_TYPE => encoder.encode(&self.expire_type).unwrap(),
				FATTR4_CHANGE => encoder.encode(&self.change).unwrap(),
				FATTR4_SIZE => encoder.encode(&self.size).unwrap(),
				FATTR4_LINK_SUPPORT => encoder.encode(&self.link_support).unwrap(),
				FATTR4_SYMLINK_SUPPORT => encoder.encode(&self.symlink_support).unwrap(),
				FATTR4_NAMED_ATTR => encoder.encode(&self.named_attr).unwrap(),
				FATTR4_FSID => encoder.encode(&self.fsid).unwrap(),
				FATTR4_UNIQUE_HANDLES => encoder.encode(&self.unique_handles).unwrap(),
				FATTR4_LEASE_TIME => encoder.encode(&self.lease_time).unwrap(),
				FATTR4_RDATTR_ERROR => encoder.encode(&self.rdattr_error).unwrap(),
				FATTR4_FILEHANDLE => encoder.encode(&self.file_handle).unwrap(),
				FATTR4_ACL => encoder.encode(&self.acl).unwrap(),
				FATTR4_ACLSUPPORT => encoder.encode(&self.aclsupport).unwrap(),
				FATTR4_ARCHIVE => encoder.encode(&self.archive).unwrap(),
				FATTR4_CANSETTIME => encoder.encode(&self.cansettime).unwrap(),
				FATTR4_CASE_INSENSITIVE => encoder.encode(&self.case_insensitive).unwrap(),
				FATTR4_CASE_PRESERVING => encoder.encode(&self.case_preserving).unwrap(),
				FATTR4_CHOWN_RESTRICTED => encoder.encode(&self.chown_restricted).unwrap(),
				FATTR4_FILEID => encoder.encode(&self.fileid).unwrap(),
				FATTR4_FILES_AVAIL => encoder.encode(&self.files_avail).unwrap(),
				FATTR4_FILES_FREE => encoder.encode(&self.files_free).unwrap(),
				FATTR4_FILES_TOTAL => encoder.encode(&self.files_total).unwrap(),
				FATTR4_HIDDEN => encoder.encode(&self.hidden).unwrap(),
				FATTR4_HOMOGENEOUS => encoder.encode(&self.homogeneous).unwrap(),
				FATTR4_MAXFILESIZE => encoder.encode(&self.maxfilesize).unwrap(),
				FATTR4_MAXLINK => encoder.encode(&self.maxlink).unwrap(),
				FATTR4_MAXNAME => encoder.encode(&self.maxname).unwrap(),
				FATTR4_MAXREAD => encoder.encode(&self.maxread).unwrap(),
				FATTR4_MAXWRITE => encoder.encode(&self.maxwrite).unwrap(),
				FATTR4_MIMETYPE => encoder.encode(&self.mimetype).unwrap(),
				FATTR4_MODE => encoder.encode(&self.mode).unwrap(),
				FATTR4_FS_LOCATIONS => encoder.encode(&self.fs_locations).unwrap(),
				FATTR4_NO_TRUNC => encoder.encode(&self.no_trunc).unwrap(),
				FATTR4_NUMLINKS => encoder.encode(&self.numlinks).unwrap(),
				FATTR4_OWNER => encoder.encode(&self.owner).unwrap(),
				FATTR4_OWNER_GROUP => encoder.encode(&self.owner_group).unwrap(),
				FATTR4_QUOTA_AVAIL_HARD => encoder.encode(&self.quota_avail_hard).unwrap(),
				FATTR4_QUOTA_AVAIL_SOFT => encoder.encode(&self.quota_avail_soft).unwrap(),
				FATTR4_QUOTA_USED => encoder.encode(&self.quota_used).unwrap(),
				FATTR4_RAWDEV => encoder.encode(&self.rawdev).unwrap(),
				FATTR4_SPACE_AVAIL => encoder.encode(&self.space_avail).unwrap(),
				FATTR4_SPACE_FREE => encoder.encode(&self.space_free).unwrap(),
				FATTR4_SPACE_TOTAL => encoder.encode(&self.space_total).unwrap(),
				FATTR4_SPACE_USED => encoder.encode(&self.space_used).unwrap(),
				FATTR4_SYSTEM => encoder.encode(&self.system).unwrap(),
				FATTR4_TIME_ACCESS => encoder.encode(&self.time_access).unwrap(),
				FATTR4_TIME_BACKUP => encoder.encode(&self.time_backup).unwrap(),
				FATTR4_TIME_CREATE => encoder.encode(&self.time_create).unwrap(),
				FATTR4_TIME_DELTA => encoder.encode(&self.time_delta).unwrap(),
				FATTR4_TIME_METADATA => encoder.encode(&self.time_metadata).unwrap(),
				FATTR4_TIME_MODIFY => encoder.encode(&self.time_modify).unwrap(),
				FATTR4_MOUNTED_ON_FILEID => encoder.encode(&self.mounted_on_fileid).unwrap(),
				_ => (),
			};
		}
		buf
	}
}
