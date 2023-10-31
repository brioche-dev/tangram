use crate::nfs::{
	state::NodeKind,
	types::{nfsstat4, stateid4, CLOSE4args, CLOSE4res},
	Context, Server,
};

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_close(&self, ctx: &Context, arg: CLOSE4args) -> CLOSE4res {
		let Some(fh) = ctx.current_file_handle else {
			return CLOSE4res::Default(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let Some(node) = self.get_node(fh).await else {
			tracing::error!(?fh, "Unknown filehandle.");
			return CLOSE4res::Default(nfsstat4::NFS4ERR_BADHANDLE);
		};

		if let NodeKind::File { .. } = &node.kind {
			let mut state = self.state.write().await;
			if state.blob_readers.remove(&arg.open_stateid).is_none() {
				return CLOSE4res::Default(nfsstat4::NFS4ERR_BAD_STATEID);
			}
		}

		let stateid = stateid4 {
			seqid: arg.seqid,
			other: [0; 12],
		};

		CLOSE4res::NFS4_OK(stateid)
	}
}
