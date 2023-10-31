use crate::nfs::{
	types::{nfsstat4, RENEW4args, RENEW4res},
	Server,
};

impl Server {
	#[tracing::instrument(skip(self))]
	pub fn handle_renew(&self, _arg: RENEW4args) -> RENEW4res {
		RENEW4res {
			status: nfsstat4::NFS4_OK,
		}
	}
}
