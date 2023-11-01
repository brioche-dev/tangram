use crate::nfs::{
	state::NodeKind,
	types::{nfsstat4, READ4args, READ4res, READ4resok},
	Context, Server,
};
use num::ToPrimitive;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_read(&self, ctx: &Context, arg: READ4args) -> READ4res {
		let Some(fh) = ctx.current_file_handle else {
			return READ4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};
		let Some(node) = self.get_node(fh).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return READ4res::Error(nfsstat4::NFS4ERR_BADHANDLE);
		};

		// RFC 7530 16.23.4:
		// "If the current file handle is not a regular file, an error will be returned to the client. In the case where the current filehandle represents a directory, NFS4ERR_ISDIR is returned; otherwise, NFS4ERR_INVAL is returned."
		let file_size = match &node.kind {
			NodeKind::Directory { .. } | NodeKind::Root { .. } => {
				return READ4res::Error(nfsstat4::NFS4ERR_ISDIR)
			},
			NodeKind::Symlink { .. } => return READ4res::Error(nfsstat4::NFS4ERR_INVAL),
			NodeKind::File { size, .. } => *size,
		};

		// It is allowed for clients to attempt to read past the end of a file, in which case the server returns an empty file.
		if arg.offset >= file_size {
			return READ4res::NFS4_OK(READ4resok {
				eof: true,
				data: vec![],
			});
		}

		let state = self.state.read().await;
		let Some(reader) = state.blob_readers.get(&arg.stateid).cloned() else {
			tracing::error!(?arg.stateid, "No reader is registered for the given id.");
			return READ4res::Error(nfsstat4::NFS4ERR_BAD_STATEID);
		};

		let mut reader = reader.write().await;

		if let Err(e) = reader.seek(std::io::SeekFrom::Start(arg.offset)).await {
			tracing::error!(?e, "Failed to seek.");
			return READ4res::Error(e.into());
		}

		let read_size = arg
			.count
			.to_u64()
			.unwrap()
			.min(file_size - arg.offset)
			.to_usize()
			.unwrap();
		let mut data = vec![0u8; read_size];
		if let Err(e) = reader.read_exact(&mut data).await {
			tracing::error!(?e, "Failed to read from file.");
			return READ4res::Error(e.into());
		}

		let eof = (arg.offset + arg.count.to_u64().unwrap()) >= file_size;
		READ4res::NFS4_OK(READ4resok { eof, data })
	}
}
