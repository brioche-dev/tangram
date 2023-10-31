use crate::nfs::{
	types::{nfsstat4, SETCLIENTID_CONFIRM4args, SETCLIENTID_CONFIRM4res},
	Server,
};

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_set_client_id_confirm(
		&self,
		arg: SETCLIENTID_CONFIRM4args,
	) -> SETCLIENTID_CONFIRM4res {
		let mut state = self.state.write().await;
		for client in state.clients.values_mut() {
			if client.server_id == arg.clientid {
				if client.server_verifier == arg.setclientid_confirm {
					client.confirmed = true;
					return SETCLIENTID_CONFIRM4res {
						status: nfsstat4::NFS4_OK,
					};
				}
				return SETCLIENTID_CONFIRM4res {
					status: nfsstat4::NFS4ERR_CLID_INUSE,
				};
			}
		}
		SETCLIENTID_CONFIRM4res {
			status: nfsstat4::NFS4ERR_STALE_CLIENTID,
		}
	}
}
