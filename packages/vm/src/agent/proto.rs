//! Defines the protocol used by the host and guest-agent.

use anyhow::Context;
use derive_more::{Display, From};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{instrument, trace};

pub type Error = serde_error::Error;
pub type Result<T> = std::result::Result<T, Error>;

pub trait ReadWrite: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T> ReadWrite for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

/// The protocol version number.
///
/// We'll assume no backwards compatibility---if the host and guest-agent have two different
/// version numbers, the guest-agent will be updated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, From, Display)]
pub struct Version(pub u64);

impl Version {
	/// The current protocol version number.
	///
	/// This version number is incremented when there are any changes to the communication protocol
	/// between the host and the guest agent. We do this so that we know when the guest agent in an old
	/// VM image needs to be updated.
	pub const CURRENT: Version = Version(4); // v4 added GetNetworkInfo
}

/// Message sent by the agent as soon as it starts.
///
/// NOTE: Do not add anything else to this message. We need to be sure that hosts can understand
/// it, even if the agent has a different version. This message needs to be the same across
/// versions.
///
/// This message is the only message sent by the agent without any request from the host. We
/// use it to detect when the guest has booted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AgentHandshake {
	pub version: Version,
}

/// Associate a response type to every request.
///
/// This is, essentially, a simple way of implementing
/// [session types](https://en.wikipedia.org/wiki/Session_type).
pub trait Respond: Serialize + DeserializeOwned {
	type Response: Serialize + DeserializeOwned + Debug;
}

/// A request that can be sent to the guest agent.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum Request {
	Heartbeat(Heartbeat),
	RunCommand(RunCommand),
	GetNetworkInfo(GetNetworkInfo),
}

/// Check that the guest agent is still alive. The agent will respond with the same token as the
/// request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
	pub nonce: u64,
}
impl Respond for Heartbeat {
	type Response = u64;
}

/// Run a command as a subprocess, wait for it to complete, and return its captured stdout and
/// stderr.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCommand {
	/// ID of a user to run the command as. If None, runs as whichever the user the guest agent
	/// runs as (which is usually root).
	pub uid: Option<u32>,

	/// Path to the executable inside the guest.
	pub executable: PathBuf,

	/// Command arguments
	pub args: Vec<OsString>,

	/// Command environment variables
	pub env: HashMap<OsString, OsString>,

	/// Data to write to the stdin of the process.
	pub stdin: Vec<u8>,
}

/// Results of running a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunCommandResponse {
	pub exit_code: Option<i32>,
	pub exit_signal: Option<i32>,
	pub stdout: Vec<u8>,
	pub stderr: Vec<u8>,
}

impl Respond for RunCommand {
	type Response = Result<RunCommandResponse>;
}

/// Retrieve network information from the guest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNetworkInfo {}

/// A guest's network information
#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNetworkInfoResponse {
	#[serde_as(as = "DisplayFromStr")]
	pub local_ip: std::net::IpAddr,
	pub interfaces: Vec<NetworkInterfaceInfo>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceInfo {
	/// Name of the interface
	pub name: String,

	/// IP address of the interface
	#[serde_as(as = "DisplayFromStr")]
	pub ip: std::net::IpAddr,
}

impl Respond for GetNetworkInfo {
	type Response = Result<GetNetworkInfoResponse>;
}

/// Write a packet (length-prefixed, bincode-encoded byte buffer) to the given socket.
#[instrument(level="DEBUG", skip_all, fields(message=?message))]
pub async fn write_packet<T>(sock: &mut dyn ReadWrite, message: T) -> anyhow::Result<()>
where
	T: Serialize + Debug,
{
	// Encode the message.
	let buf = bincode::serialize(&message).context("failed to serialize data")?;

	// Write the length of the message as a header.
	// Length is written as a 64-bit big-endian unsigned integer.
	trace!(packet_len = buf.len());
	sock.write_u64(buf.len().try_into().unwrap())
		.await
		.context("failed to write header")?;

	trace!(buf=?buf);

	// Write the contents of the message.
	sock.write_all(&buf).await.context("failed to write data")?;

	Ok(())
}

/// Read a packet (length-prefixed, bincode-encoded byte buffer) from the given socket, or return
/// None if the socket has been closed.
#[instrument(level = "DEBUG", skip_all)]
pub async fn read_packet_or_eof<T>(sock: &mut dyn ReadWrite) -> anyhow::Result<Option<T>>
where
	T: serde::de::DeserializeOwned + Debug,
{
	// Read the length of the packet from the socket.
	// Length is read as a 64-bit big-endian unsigned integer.
	let len: usize = match sock.read_u64().await {
		Ok(size) => size.try_into().expect("u64 usize mismatch"),

		// If the socket has been closed after the last packet, return None.
		Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
			return Ok(None);
		},

		// Otherwise, pass through read errors to the caller.
		Err(e) => Err(e).context("failed to read packet length")?,
	};
	trace!(packet_len = len);

	// Read the data from the socket.
	let mut buf = vec![0u8; len];
	sock.read_exact(&mut buf)
		.await
		.context("failed to read data")?;
	trace!(buf=?buf);

	// Decode the message from the packet contents.
	let message = bincode::deserialize::<T>(&buf).context("failed to deserialize data")?;
	trace!(msg=?message);

	Ok(Some(message))
}

/// Read a packet (length-prefixed, bincode-encoded byte buffer) from the given socket, or return
/// None if the socket has been closed.
pub async fn read_packet<T>(sock: &mut dyn ReadWrite) -> anyhow::Result<T>
where
	T: serde::de::DeserializeOwned + Debug,
{
	let packet = read_packet_or_eof(sock)
		.await?
		.context("cannot read packet from closed socket")?;
	Ok(packet)
}
