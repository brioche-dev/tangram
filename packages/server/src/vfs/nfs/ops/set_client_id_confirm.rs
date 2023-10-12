use crate::vfs::nfs::{server::Server, types::*, xdr::FromXdr};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Arg {
	pub clientid: u64,
	pub verifier: [u8; NFS4_VERIFIER_SIZE],
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_set_client_id_confirm(&self, arg: Arg) -> i32 {
		let mut state = self.state.write().await;
		for client in state.clients.values_mut() {
			if client.server_id == arg.clientid {
				if client.server_verifier == arg.verifier {
					client.confirmed = true;
					return NFS4_OK;
				}
				return NFS4ERR_CLID_INUSE;
			}
		}
		NFS4ERR_STALE_CLIENTID
	}
}

impl FromXdr for Arg {
	fn decode(
		decoder: &mut crate::vfs::nfs::xdr::Decoder<'_>,
	) -> Result<Self, crate::vfs::nfs::xdr::Error> {
		let clientid = decoder.decode()?;
		let verifier = decoder.decode_n()?;
		Ok(Self { clientid, verifier })
	}
}
