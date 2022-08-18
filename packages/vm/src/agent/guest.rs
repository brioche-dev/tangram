use super::proto::{self, read_packet, write_packet};
use anyhow::{Context, Result};
use serde::Serialize;
use std::os::unix::process::ExitStatusExt;
use std::{fmt::Debug, process::Stdio};
use tokio::io::AsyncWriteExt;
use tokio::process;
use tracing::{debug, info, instrument};

/// Server running in the guest agent, responding to requests from the host.
pub struct Server {
	sock: Box<dyn proto::ReadWrite>,
}

impl Server {
	/// Creates a server and sends the handshake message.
	#[instrument(name = "Server::new", skip_all)]
	pub async fn new<RW>(mut sock: RW) -> Result<Server>
	where
		RW: proto::ReadWrite + 'static,
	{
		// Send the handshake message.
		let handshake = proto::AgentHandshake {
			version: proto::Version::CURRENT,
		};
		debug!(version = %handshake.version, "sending agent handshake");
		write_packet(&mut sock, &handshake)
			.await
			.context("failed to send agent handshake")?;

		Ok(Server {
			sock: Box::new(sock),
		})
	}

	/// Handle all requests to the agent.
	#[instrument(name = "Server::handle", skip_all)]
	pub async fn handle(&mut self) -> Result<()> {
		use proto::Request;

		loop {
			// Read a message.
			let request = read_packet::<proto::Request>(self.sock.as_mut())
				.await
				.context("failed to read request")?;
			info!(request=?request);

			match request {
				Request::Heartbeat(r) => self.respond(r.nonce).await?,
				Request::RunCommand(r) => {
					let resp = self.run_command(&r).await.map_err(to_proto_err);
					self.respond(resp).await?;
				},
				Request::GetNetworkInfo(r) => {
					let resp = self.get_network_info(&r).await.map_err(to_proto_err);
					self.respond(resp).await?;
				},
			};
		}
	}

	/// Send a message response over the socket.
	#[instrument(name = "Server::respond", skip_all)]
	async fn respond<T>(&mut self, response: T) -> Result<()>
	where
		T: Serialize + Debug,
	{
		info!(response=?response);
		write_packet(self.sock.as_mut(), &response)
			.await
			.context("failed to write response")?;
		Ok(())
	}

	/// Handle a [`proto::GetNetworkInfo`] request.
	#[instrument(name = "Server::get_network_info", skip_all)]
	async fn get_network_info(
		&mut self,
		_msg: &proto::GetNetworkInfo,
	) -> Result<proto::GetNetworkInfoResponse> {
		// Using a netlink socket, gather a list of all the AF_INET interfaces on this system, and
		// get their IP addresses.
		let ifas = tokio::task::block_in_place(local_ip_address::list_afinet_netifas)
			.context("failed to get network interfaces")?;
		let interfaces = ifas
			.into_iter()
			.map(|x| proto::NetworkInterfaceInfo { name: x.0, ip: x.1 })
			.filter(|info| !info.ip.is_loopback()) // Exclude loopback
			.collect();

		let local_ip = tokio::task::block_in_place(local_ip_address::local_ip)
			.context("failed to get local IP")?;

		Ok(proto::GetNetworkInfoResponse {
			local_ip,
			interfaces,
		})
	}

	/// Handle a [`proto::RunCommand`] request.
	#[instrument(
		name = "Server::run_command",
		skip_all,
		fields(executable=?msg.executable)
	)]
	async fn run_command(&mut self, msg: &proto::RunCommand) -> Result<proto::RunCommandResponse> {
		let mut cmd = process::Command::new(&msg.executable);

		// Set the child arguments.
		cmd.args(&msg.args);

		// Set up the child environment.
		// NOTE: we don't inherit any environment from this process
		cmd.env_clear();
		cmd.envs(&msg.env);

		// If requested, set the user ID to run the child as.
		if let Some(uid) = msg.uid {
			cmd.uid(uid);
		}

		// Pipe all stdio back to this process.
		cmd.stdin(Stdio::piped());
		cmd.stdout(Stdio::piped());
		cmd.stderr(Stdio::piped());

		// Spawn the subprocess.
		let mut child = cmd.spawn().context("failed to spawn subprocess")?;

		// Write the stdin from the message.
		let mut stdin = child.stdin.take().unwrap();
		stdin
			.write_all(&msg.stdin)
			.await
			.context("failed to write stdin of subprocess")?;
		stdin.shutdown().await?;
		drop(stdin);

		// Wait for the child to terminate.
		let output = child
			.wait_with_output()
			.await
			.context("failed to wait for subprocess exit")?;

		Ok(proto::RunCommandResponse {
			exit_code: output.status.code(),
			exit_signal: output.status.signal(),
			stdout: output.stdout,
			stderr: output.stderr,
		})
	}
}

/// Convert a local error to a serializable [`proto::Error`]
fn to_proto_err<E>(err: E) -> proto::Error
where
	E: AsRef<dyn std::error::Error + 'static>,
{
	proto::Error::new(err.as_ref())
}
