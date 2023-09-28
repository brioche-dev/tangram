use crate::{
	template,
	vfs::nfs::{
		server::{Context, Server},
		state::NodeKind,
		types::*,
		xdr::ToXdr,
	},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResOp {
	Ok(Vec<u8>),
	Err(i32),
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_readlink(&self, ctx: &Context) -> ResOp {
		let Some(fh) = ctx.current_file_handle else {
			return ResOp::Err(NFS4ERR_NOFILEHANDLE);
		};
		let Some(node) = self.get_node(fh.node).await else {
			return ResOp::Err(NFS4ERR_NOENT);
		};
		let NodeKind::Symlink { symlink } = &node.kind else {
			return ResOp::Err(NFS4ERR_INVAL);
		};
		let Ok(target) = symlink.target(&self.client).await else {
			return ResOp::Err(NFS4ERR_IO);
		};
		let mut response = String::new();
		for component in target.components() {
			match component {
				template::Component::String(string) => {
					response.extend(string.chars());
				},
				template::Component::Artifact(artifact) => {
					let Ok(id) = artifact.id(&self.client).await else {
						return ResOp::Err(NFS4ERR_IO);
					};
					for _ in 0..node.depth() {
						response.push_str("../");
					}
					response.push_str(&id.to_string());
				},
				template::Component::Placeholder(_) => {
					tracing::error!("Cannot render placeholders in symlinks in the tangram VFS.");
					return ResOp::Err(NFS4ERR_IO);
				},
			}
		}

		ResOp::Ok(response.into_bytes())
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

impl ToXdr for ResOp {
	fn encode<W>(
		&self,
		encoder: &mut crate::vfs::nfs::xdr::Encoder<W>,
	) -> Result<(), crate::vfs::nfs::xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_int(self.status())?;
		if let Self::Ok(linktext) = self {
			encoder.encode_opaque(linktext)?;
		}
		Ok(())
	}
}
