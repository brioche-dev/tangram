use crate::nfs::{
	state::NodeKind,
	types::{
		change_info4, nfsstat4, open_claim4, open_delegation4, open_delegation_type4, stateid4,
		OPEN4args, OPEN4res, OPEN4resok, bitmap4,
	},
	Context, Server,
};
use std::sync::Arc;
use tokio::sync::RwLock;

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_open(&self, ctx: &mut Context, arg: OPEN4args) -> OPEN4res {
		let Some(fh) = ctx.current_file_handle else {
			return OPEN4res::Error(nfsstat4::NFS4ERR_NOFILEHANDLE);
		};

		let fh = match arg.claim {
			open_claim4::CLAIM_NULL(name) => {
				let Ok(name) = std::str::from_utf8(&name) else {
					return OPEN4res::Error(nfsstat4::NFS4ERR_NOENT);
				};
				match self.lookup(fh, name).await {
					Ok(fh) => fh,
					Err(e) => return OPEN4res::Error(e),
				}
			},
			open_claim4::CLAIM_PREVIOUS(open_delegation_type4::OPEN_DELEGATE_NONE) => fh,
			_ => return OPEN4res::Error(nfsstat4::NFS4ERR_IO),
		};

		ctx.current_file_handle = Some(fh);
		let stateid = stateid4 {
			seqid: arg.seqid + 1,
			other: [0; 12],
		};

		if let NodeKind::File { file, .. } = &self.get_node(fh).await.unwrap().kind {
			let Ok(blob) = file.contents(self.client.as_ref()).await else {
				tracing::error!("Failed to get file's content.");
				return OPEN4res::Error(nfsstat4::NFS4ERR_IO);
			};
			let Ok(reader) = blob.reader(self.client.as_ref()).await else {
				tracing::error!("Failed to create blob reader.");
				return OPEN4res::Error(nfsstat4::NFS4ERR_IO);
			};
			self.state
				.write()
				.await
				.blob_readers
				.insert(stateid, Arc::new(RwLock::new(reader)));
		}

		let cinfo = change_info4 {
			atomic: false,
			before: 0,
			after: 0,
		};

		let rflags = 0;
		let attrset = bitmap4(vec![]);
		let delegation = open_delegation4::OPEN_DELEGATE_NONE;
		let resok = OPEN4resok {
			stateid,
			cinfo,
			rflags,
			attrset,
			delegation,
		};
		OPEN4res::NFS4_OK(resok)
	}
}
