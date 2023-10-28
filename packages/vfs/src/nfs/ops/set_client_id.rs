use crate::nfs::{server::Server, state::ClientData, types::*, xdr};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Arg {
	pub client: ClientId,
	pub callback: CallbackClient,
	pub callback_ident: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ClientId {
	pub verifier: [u8; NFS4_VERIFIER_SIZE],
	pub id: Vec<u8>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CallbackClient {
	pub cb_program: u32,
	pub cb_location: ClientAddr,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClientAddr {
	r_netid: String,
	r_addr: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ResOp {
	Ok {
		clientid: u64,
		setclientid_confirm: [u8; NFS4_VERIFIER_SIZE],
	},
	ClientAddrInUse(ClientAddr),
	Err(i32),
}

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_set_client_id(&self, arg: Arg) -> ResOp {
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
			return ResOp::Ok {
				clientid: server_id,
				setclientid_confirm: server_verifier,
			};
		};

		let conditions = [
			client.confirmed,
			client.client_verifier == arg.client.verifier,
			client.callback == arg.callback,
			client.callback_ident == arg.callback_ident,
		];

		if !conditions.into_iter().all(|c| c) {
			// TODO: extend to handle any other cases.
			ResOp::Err(NFS4ERR_IO)
		} else {
			let clientid = client.server_id;
			let setclientid_confirm = client.server_verifier;
			ResOp::Ok {
				clientid,
				setclientid_confirm,
			}
		}
	}
}

impl xdr::FromXdr for Arg {
	fn decode(decoder: &mut xdr::Decoder<'_>) -> Result<Self, xdr::Error> {
		let client = decoder.decode()?;
		tracing::info!(?client, "Decoded client.");
		let callback = decoder.decode()?;
		tracing::info!(?callback, "Decoded callback.");
		let callback_ident = decoder.decode()?;
		tracing::info!(?callback_ident, "Decoded callback ident.");

		Ok(Self {
			client,
			callback,
			callback_ident,
		})
	}
}

impl xdr::FromXdr for ClientId {
	fn decode(decoder: &mut xdr::Decoder<'_>) -> Result<Self, xdr::Error> {
		let verifier = decoder.decode_n()?;
		let id = decoder.decode()?;
		Ok(Self { verifier, id })
	}
}

impl xdr::FromXdr for CallbackClient {
	fn decode(decoder: &mut xdr::Decoder<'_>) -> Result<Self, xdr::Error> {
		let cb_program = decoder.decode()?;
		let cb_location = decoder.decode()?;
		Ok(Self {
			cb_program,
			cb_location,
		})
	}
}

impl xdr::FromXdr for ClientAddr {
	fn decode(decoder: &mut xdr::Decoder<'_>) -> Result<Self, xdr::Error> {
		let r_netid = decoder.decode()?;
		let r_addr = decoder.decode()?;
		Ok(Self { r_netid, r_addr })
	}
}

impl xdr::ToXdr for ResOp {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		match self {
			Self::Ok {
				clientid,
				setclientid_confirm,
			} => {
				encoder.encode_int(NFS4_OK)?;
				encoder.encode(clientid)?;
				encoder.encode_n(*setclientid_confirm)?;
			},
			Self::ClientAddrInUse(addr) => {
				encoder.encode_int(NFS4ERR_CLID_INUSE)?;
				encoder.encode(addr)?;
			},
			Self::Err(e) => {
				encoder.encode(e)?;
			},
		}
		Ok(())
	}
}

impl xdr::ToXdr for ClientId {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode_n(self.verifier)?;
		encoder.encode(&self.id)?;
		Ok(())
	}
}

impl xdr::ToXdr for ClientAddr {
	fn encode<W>(&self, encoder: &mut xdr::Encoder<W>) -> Result<(), xdr::Error>
	where
		W: std::io::Write,
	{
		encoder.encode(&self.r_netid)?;
		encoder.encode(&self.r_addr)?;
		Ok(())
	}
}

impl ResOp {
	pub fn status(&self) -> i32 {
		match self {
			Self::Ok { .. } => NFS4_OK,
			Self::ClientAddrInUse(_) => NFS4ERR_CLID_INUSE,
			Self::Err(e) => *e,
		}
	}
}
