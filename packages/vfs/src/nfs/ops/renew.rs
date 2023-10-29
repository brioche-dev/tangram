use crate::nfs::{ops::set_client_id::ClientId, types::NFS4_OK, Server};

impl Server {
	#[tracing::instrument(skip(self))]
	pub fn handle_renew(&self, arg: ClientId) -> i32 {
		NFS4_OK
	}
}
