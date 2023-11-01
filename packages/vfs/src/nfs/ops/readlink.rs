use crate::nfs::{
	state::NodeKind,
	types::{nfsstat4, READLINK4res, READLINK4resok},
	Context, Server,
};
use tangram_client as tg;

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_readlink(&self, ctx: &Context) -> READLINK4res {
		let Some(fh) = ctx.current_file_handle else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};
		let Some(node) = self.get_node(fh).await else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_NOENT);
		};
		let NodeKind::Symlink { symlink } = &node.kind else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_INVAL);
		};
		let Ok(target) = symlink.target(self.client.as_ref()).await else {
			return READLINK4res::Error(nfsstat4::NFS4ERR_IO);
		};
		let mut response = String::new();
		for component in target.components() {
			match component {
				tg::template::Component::String(string) => {
					response.push_str(string);
				},
				tg::template::Component::Artifact(artifact) => {
					let Ok(id) = artifact.id(self.client.as_ref()).await else {
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
}
