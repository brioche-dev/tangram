use crate::nfs::{
	state::NodeKind,
	types::{
		nfsstat4, ACCESS4args, ACCESS4res, ACCESS4resok, ACCESS4_EXECUTE, ACCESS4_LOOKUP,
		ACCESS4_READ,
	},
	Context, Server,
};

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_access(&self, ctx: &Context, arg: ACCESS4args) -> ACCESS4res {
		let Some(fh) = ctx.current_file_handle else {
			return ACCESS4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return ACCESS4res::Error(nfsstat4::NFS4ERR_BADHANDLE);
		};

		let access = match &node.kind {
			NodeKind::Root { .. } => ACCESS4_EXECUTE | ACCESS4_READ | ACCESS4_LOOKUP,
			NodeKind::Directory { .. } => ACCESS4_EXECUTE | ACCESS4_READ | ACCESS4_LOOKUP,
			NodeKind::Symlink { .. } => ACCESS4_READ,
			NodeKind::File { file, .. } => {
				let is_executable = match file.executable(self.client.as_ref()).await {
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
}
