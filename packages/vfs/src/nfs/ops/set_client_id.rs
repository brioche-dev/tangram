use crate::nfs::{
	state::ClientData,
	types::{nfsstat4, SETCLIENTID4args, SETCLIENTID4res, SETCLIENTID4resok},
	Server,
};

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_set_client_id(&self, arg: SETCLIENTID4args) -> SETCLIENTID4res {
		let mut state = self.state.write().await;
		let Some(client) = state.clients.get(&arg.client.id) else {
			let (server_id, server_verifier) = state.new_client_data();
			let record = ClientData {
				server_id,
				client_verifier: arg.client.verifier,
				server_verifier,
				callback: arg.callback,
				callback_ident: arg.callback_ident,
				confirmed: false,
			};
			let _ = state.clients.insert(arg.client.id, record);
			return SETCLIENTID4res::NFS4_OK(SETCLIENTID4resok {
				clientid: server_id,
				setclientid_confirm: server_verifier,
			});
		};

		let conditions = [
			client.confirmed,
			client.client_verifier == arg.client.verifier,
			client.callback == arg.callback,
			client.callback_ident == arg.callback_ident,
		];

		if !conditions.into_iter().all(|c| c) {
			// TODO: extend to handle any other cases.
			SETCLIENTID4res::Default(nfsstat4::NFS4ERR_IO)
		} else {
			let clientid = client.server_id;
			let setclientid_confirm = client.server_verifier;
			SETCLIENTID4res::NFS4_OK(SETCLIENTID4resok {
				clientid,
				setclientid_confirm,
			})
		}
	}
}
