use crate::nfs::{state::NodeKind, types::*, xdr, Context, Server};

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
			if state.blob_readers.remove(&arg.open_stateid).is_none() {
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

impl xdr::ToXdr for Arg {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_uint(self.seqid)?;
		encoder.encode(&self.open_stateid)?;
		Ok(())
	}
}

impl xdr::FromXdr for Arg {
	fn decode(decoder: &mut xdr::Decoder<'_>) -> Result<Self, xdr::Error> {
		let seqid = decoder.decode_uint()?;
		let open_stateid = decoder.decode()?;
		Ok(Self {
			seqid,
			open_stateid,
		})
	}
}

impl xdr::ToXdr for ResOp {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
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
