use super::proto::{self, read_packet, write_packet};
use anyhow::{bail, Context, Result};
use tracing::{debug, instrument, trace};

/// Client used by the host to talk to the guest agent.
pub struct Client {
	/// Communication socket with the guest agent.
	sock: Box<dyn proto::ReadWrite>,
}

impl Client {
	/// Create a new client, waiting for the handshake from the guest agent.
	#[instrument(level = "DEBUG", name = "Client::connect", skip_all)]
	pub async fn connect<RW>(mut sock: RW) -> Result<Client>
	where
		RW: proto::ReadWrite + 'static,
	{
		let handshake = read_packet::<proto::AgentHandshake>(&mut sock)
			.await
			.context("failed to read agent handshake")?;

		if handshake.version != proto::Version::CURRENT {
			let guest_agent_version = handshake.version;
			let host_version = proto::Version::CURRENT;
			bail!("version mismatch between guest agent ({guest_agent_version}) and host ({host_version})");
		}
		trace!(handshake_version=%handshake.version);

		Ok(Client {
			sock: Box::new(sock),
		})
	}

	/// Make a request of the guest agent.
	#[instrument(level = "DEBUG", name = "Client::request", skip_all)]
	pub async fn request<T>(&mut self, request: T) -> Result<T::Response>
	where
		T: proto::Respond + std::fmt::Debug,
		proto::Request: From<T>,
	{
		debug!(request=?request);

		// Send the request.
		let request = proto::Request::from(request);
		write_packet(self.sock.as_mut(), request)
			.await
			.context("failed to send request")?;

		// Read the response.
		let response = read_packet::<T::Response>(self.sock.as_mut())
			.await
			.context("failed to read response")?;

		debug!(response=?response);

		Ok(response)
	}
}
