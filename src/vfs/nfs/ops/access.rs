use crate::vfs::nfs::{
	server::{Context, Server},
	state::NodeKind,
	types::*,
	xdr::ToXdr,
};

pub type Arg = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResOp {
	Ok { supported: u32, access: u32 },
	Err(i32),
}

pub mod flags {
	pub const ACCESS4_READ: u32 = 0x00000001;
	pub const ACCESS4_LOOKUP: u32 = 0x00000002;
	pub const ACCESS4_MODIFY: u32 = 0x00000004;
	pub const ACCESS4_EXTEND: u32 = 0x00000008;
	pub const ACCESS4_DELETE: u32 = 0x00000010;
	pub const ACCESS4_EXECUTE: u32 = 0x00000020;
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_access(&self, ctx: &Context, arg: Arg) -> ResOp {
		let Some(fh) = ctx.current_file_handle else {
			return ResOp::Err(NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh.node).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return ResOp::Err(NFS4ERR_BADHANDLE);
		};

		let access = match &node.kind {
			NodeKind::Root { .. } => {
				flags::ACCESS4_EXECUTE | flags::ACCESS4_READ | flags::ACCESS4_LOOKUP
			},
			NodeKind::Directory { .. } => {
				flags::ACCESS4_EXECUTE | flags::ACCESS4_READ | flags::ACCESS4_LOOKUP
			},
			NodeKind::Symlink { .. } => flags::ACCESS4_READ,
			NodeKind::File { file, .. } => {
				let is_executable = match file.executable(&self.client).await {
					Ok(b) => b,
					Err(e) => {
						tracing::error!(?e, "Failed to lookup executable bit for file.");
						return ResOp::Err(NFS4ERR_IO);
					},
				};
				if is_executable {
					flags::ACCESS4_EXECUTE | flags::ACCESS4_READ
				} else {
					flags::ACCESS4_READ
				}
			},
		};

		ResOp::Ok {
			supported: arg & access,
			access,
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

impl ToXdr for ResOp {
	fn encode<W>(
		&self,
		encoder: &mut crate::vfs::nfs::xdr::Encoder<W>,
	) -> Result<(), crate::vfs::nfs::xdr::Error>
	where
		W: std::io::Write,
	{
		match self {
			Self::Ok { supported, access } => {
				encoder.encode_int(NFS4_OK)?;
				encoder.encode(supported)?;
				encoder.encode(access)?;
			},
			Self::Err(e) => {
				encoder.encode_int(*e)?;
			},
		}
		Ok(())
	}
}
