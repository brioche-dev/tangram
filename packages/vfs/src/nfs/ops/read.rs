use crate::nfs::{state::NodeKind, types::*, xdr, Context, Server};
use num::ToPrimitive;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Arg {
	state_id: StateId,
	offset: u64,
	count: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ResOp {
	Ok { eof: bool, data: Vec<u8> },
	Err(i32),
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_read(&self, ctx: &Context, arg: Arg) -> ResOp {
		let Some(fh) = ctx.current_file_handle else {
			return ResOp::Err(NFS4ERR_NOFILEHANDLE);
		};
		let Some(node) = self.get_node(fh.node).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return ResOp::Err(NFS4ERR_BADHANDLE);
		};

		// RFC 7530 16.23.4:
		// "If the current file handle is not a regular file, an error will be returned to the client. In the case where the current filehandle represents a directory, NFS4ERR_ISDIR is returned; otherwise, NFS4ERR_INVAL is returned."
		let file_size = match &node.kind {
			NodeKind::Directory { .. } | NodeKind::Root { .. } => return ResOp::Err(NFS4ERR_ISDIR),
			NodeKind::Symlink { .. } => return ResOp::Err(NFS4ERR_INVAL),
			NodeKind::File { size, .. } => *size,
		};

		// It is allowed for clients to attempt to read past the end of a file, in which case the server returns an empty file.
		if arg.offset >= file_size {
			return ResOp::Ok {
				eof: true,
				data: Vec::new(),
			};
		}

		let state = self.state.read().await;
		let Some(reader) = state.blob_readers.get(&arg.state_id).cloned() else {
			tracing::error!(?arg.state_id, "No reader is registered for the given id.");
			return ResOp::Err(NFS4ERR_BAD_STATEID);
		};

		let mut reader = reader.write().await;

		if let Err(e) = reader.seek(std::io::SeekFrom::Start(arg.offset)).await {
			tracing::error!(?e, "Failed to seek.");
			return ResOp::Err(e.raw_os_error().unwrap_or(NFS4ERR_IO));
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
			return ResOp::Err(e.raw_os_error().unwrap());
		}

		let eof = (arg.offset + arg.count.to_u64().unwrap()) >= file_size;
		return ResOp::Ok { eof, data };
	}
}

impl xdr::FromXdr for Arg {
	fn decode(decoder: &mut xdr::Decoder<'_>) -> Result<Self, xdr::Error> {
		let state_id = decoder.decode()?;
		let offset = decoder.decode()?;
		let count = decoder.decode()?;
		Ok(Self {
			state_id,
			offset,
			count,
		})
	}
}

impl ResOp {
	pub fn status(&self) -> i32 {
		match self {
			Self::Ok { .. } => NFS4_OK,
			Self::Err(e) => *e,
		}
	}
}

impl xdr::ToXdr for ResOp {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.status())?;
		if let Self::Ok { eof, data } = self {
			encoder.encode(eof)?;
			encoder.encode_opaque(data)?;
		}
		Ok(())
	}
}
