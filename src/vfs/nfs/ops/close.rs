use crate::vfs::nfs::{
	server::{Context, Server},
	state::NodeKind,
	types::*,
	xdr::{FromXdr, ToXdr},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Arg {
	pub seqid: u32,
	pub open_stateid: StateId,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ResOp {
	Ok(StateId),
	Err(i32),
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_close(&self, ctx: &Context, arg: Arg) -> ResOp {
		let Some(fh) = ctx.current_file_handle else {
			return ResOp::Err(NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh.node).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return ResOp::Err(NFS4ERR_BADHANDLE);
		};

		if let NodeKind::File { .. } = &node.kind {
			let mut state = self.state.write().await;
			if let None = state.blob_readers.remove(&arg.open_stateid) {
				return ResOp::Err(NFS4ERR_BAD_STATEID);
			}
		}

		ResOp::Ok(StateId {
			seqid: arg.seqid,
			other: [0; 12],
		})
	}
}

impl ResOp {
	pub fn status(&self) -> i32 {
		match self {
			Self::Ok(_) => NFS4_OK,
			Self::Err(e) => *e,
		}
	}
}

impl ToXdr for Arg {
	fn encode<W>(
		&self,
		encoder: &mut crate::vfs::nfs::xdr::Encoder<W>,
	) -> Result<(), crate::vfs::nfs::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_uint(self.seqid)?;
		encoder.encode(&self.open_stateid)?;
		Ok(())
	}
}

impl FromXdr for Arg {
	fn decode(
		decoder: &mut crate::vfs::nfs::xdr::Decoder<'_>,
	) -> Result<Self, crate::vfs::nfs::xdr::Error> {
		let seqid = decoder.decode_uint()?;
		let open_stateid = decoder.decode()?;
		Ok(Self {
			seqid,
			open_stateid,
		})
	}
}

impl ToXdr for ResOp {
	fn encode<W>(
		&self,
		encoder: &mut crate::vfs::nfs::xdr::Encoder<W>,
	) -> Result<(), crate::vfs::nfs::xdr::Error>
	where
		W: std::io::Write,
	{
		match self {
			Self::Ok(stateid) => {
				encoder.encode(&NFS4_OK)?;
				encoder.encode(stateid)?;
			},
			Self::Err(status) => encoder.encode(status)?,
		}
		Ok(())
	}
}
