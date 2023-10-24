use crate::vfs::nfs::{ops::set_client_id::ClientId, server::Server, types::NFS4_OK};

impl Server {
	#[tracing::instrument(skip(self))]
	pub fn handle_renew(&self, arg: ClientId) -> i32 {
		NFS4_OK
	}
}
