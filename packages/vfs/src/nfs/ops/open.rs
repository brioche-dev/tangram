use crate::vfs::nfs::{
	server::{Context, Server},
	state::NodeKind,
	types::*,
	xdr::{FromXdr, ToXdr},
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Arg {
	pub seqid: u32,
	pub share_access: u32,
	pub share_deny: u32,
	pub owner: OpenOwner,
	pub openhow: OpenFlags,
	pub claim: OpenClaim,
}

#[derive(Debug, Clone)]
pub enum ResOp {
	Ok {
		stateid: StateId,
		info: ChangeInfo,
		rflags: u32,
		attrset: Bitmap,
		delegation: OpenDelegation,
	},
	Err(i32),
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_open(&self, ctx: &mut Context, arg: Arg) -> ResOp {
		let Some(fh) = ctx.current_file_handle else {
			return ResOp::Err(NFS4ERR_NOFILEHANDLE);
		};

		let fh = match arg.claim {
			OpenClaim::Null(filename) => match self.lookup(fh, &filename).await {
				Ok(fh) => fh,
				Err(e) => return ResOp::Err(e),
			},
			OpenClaim::Previous(OpenDelegationType::None) => fh,
			_ => return ResOp::Err(NFS4ERR_IO),
		};

		ctx.current_file_handle = Some(fh);
		let stateid = StateId {
			seqid: arg.seqid + 1,
			other: [0; 12],
		};

		if let NodeKind::File { file, .. } = &self.get_node(fh.node).await.unwrap().kind {
			let Ok(blob) = file.contents(self.client.as_ref()).await else {
				tracing::error!("Failed to get file's content.");
				return ResOp::Err(NFS4ERR_IO);
			};
			let Ok(reader) = blob.reader(self.client.as_ref()).await else {
				tracing::error!("Failed to create blob reader.");
				return ResOp::Err(NFS4ERR_IO);
			};
			self.state
				.write()
				.await
				.blob_readers
				.insert(stateid, Arc::new(RwLock::new(reader)));
		}

		let info = ChangeInfo {
			atomic: false,
			before: 0,
			after: 0,
		};

		let rflags = 0;
		let attrset = Bitmap(vec![]);
		let delegation = OpenDelegation::None;

		ResOp::Ok {
			stateid,
			info,
			rflags,
			attrset,
			delegation,
		}
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

impl FromXdr for Arg {
	fn decode(
		decoder: &mut crate::vfs::nfs::xdr::Decoder<'_>,
	) -> Result<Self, crate::vfs::nfs::xdr::Error> {
		let seqid = decoder.decode()?;
		let share_access = decoder.decode()?;
		let share_deny = decoder.decode()?;
		let owner = decoder.decode()?;
		let openhow = decoder.decode()?;
		let claim = decoder.decode()?;
		Ok(Self {
			seqid,
			share_access,
			share_deny,
			owner,
			openhow,
			claim,
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
			Self::Ok {
				stateid,
				info,
				rflags,
				attrset,
				delegation,
			} => {
				encoder.encode_int(NFS4_OK)?;
				encoder.encode(stateid)?;
				encoder.encode(info)?;
				encoder.encode(rflags)?;
				encoder.encode(attrset)?;
				encoder.encode(delegation)?;
			},
			Self::Err(e) => encoder.encode_int(*e)?,
		};
		Ok(())
	}
}
