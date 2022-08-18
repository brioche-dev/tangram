#[cfg(target_os = "linux")]
mod linux {
	use anyhow::{bail, Context, Result};
	use camino::Utf8PathBuf;
	use clap::Parser;
	use tangram_vm::agent;
	use tokio::net::UnixStream;
	use tokio::task;
	use tokio_vsock::{SockAddr, VsockAddr, VsockListener, VsockStream};
	use tracing::{error, info, instrument};
	use vsock::VMADDR_CID_ANY;

	/// The Tangram VM guest agent.
	///
	/// This agent communicates with the host over a vsock connection.
	#[derive(Parser, Debug)]
	struct Args {
		/// Listen for vsock connections on this port
		#[clap(long)]
		port: u32,

		/// Forward vsock connections to a local unix socket.
		#[clap(long)]
		forward_unix: Vec<ForwardArg>,
	}

	#[cfg(target_os = "linux")]
	pub async fn main() -> anyhow::Result<()> {
		let args = Args::parse();

		// Log all tracing events to systemd (and eventually through to the TTY console)
		{
			use tracing_subscriber::prelude::*;
			let r = tracing_subscriber::registry();
			let r = r.with(tracing_subscriber::filter::LevelFilter::DEBUG);
			let r = r.with(tracing_subscriber::fmt::layer().with_target(false));
			if let Ok(journal_layer) = tracing_journald::layer() {
				r.with(journal_layer.boxed()).init();
			} else {
				r.init()
			}
		}

		let mut tasks = vec![];

		// Spawn a task to listen for agent connections
		tasks.push(task::spawn(accept_agent_connections(args.port)));

		// Spawn tasks to forward vsock to unix
		for forward in &args.forward_unix {
			let task = task::spawn(forward_connections(
				forward.port,
				forward.socket_path.to_owned(),
			));
			tasks.push(task);
		}

		info!("All tasks started");
		futures::future::try_join_all(tasks).await?;
		info!("All tasks completed");
		Ok(())
	}

	#[instrument(level = "INFO")]
	async fn accept_agent_connections(listen_port: u32) -> Result<()> {
		// Listen for vsock connections from the host
		let listen_cid = VMADDR_CID_ANY;
		info!(port=?listen_port, cid=?listen_cid, "Listening for agent connections");
		let mut listener = VsockListener::bind(listen_cid, listen_port)
			.with_context(|| format!("Could not bind to vsock port {}", listen_port))?;

		loop {
			// Accept a connection, or shut down the listener
			let (stream, addr) = match listener.accept().await {
				// If we got an error accepting a connection, log it.
				Err(e) => {
					error!(err=?e, "Failed to accept connection");
					continue;
				},

				// Continue with successful connection.
				Ok((stream, SockAddr::Vsock(addr))) => (stream, addr),
				Ok((_stream, _non_vsock_addr)) => {
					unreachable!("Connected to something that wasn't a vsock")
				},
			};

			info!(
				cid = addr.cid(),
				port = &addr.port(),
				"Accepted agent connection from host"
			);

			// Spawn a task to handle each connection
			tokio::task::spawn(async move {
				let _ = handle_agent_connection(stream, addr)
					.await
					.map_err(|e| error!(err = ?e, "Error handling host connection"));
			});
		}
	}

	/// Task to handle connections from the host.
	#[instrument(level = "INFO", skip_all, fields(?addr))]
	async fn handle_agent_connection(stream: VsockStream, addr: VsockAddr) -> Result<()> {
		// Create a guest agent server
		let mut server = agent::guest::Server::new(stream)
			.await
			.context("Could not start guest server")?;

		// Handle the request with it
		server
			.handle()
			.await
			.context("Error while serving request")?;

		Ok(())
	}

	/// Task to forward vsock connections to a unix socket inside the VM
	#[instrument(level = "INFO")]
	async fn forward_connections(listen_port: u32, socket_path: Utf8PathBuf) -> Result<()> {
		let listen_cid = VMADDR_CID_ANY;
		info!(port=?listen_port, cid=?listen_cid, %socket_path, "Forwarding connections to unix socket");
		let mut listener = VsockListener::bind(listen_cid, listen_port)
			.with_context(|| format!("Could not bind to vsock port {}", listen_port))?;

		loop {
			// Accept a connection, or shut down the listener
			let (stream, addr) = match listener.accept().await {
				// If we got an error accepting a connection, log it.
				Err(e) => {
					error!(err=?e, "Failed to accept connection");
					continue;
				},

				// Continue with successful connection.
				Ok((stream, SockAddr::Vsock(addr))) => (stream, addr),
				Ok((_stream, _non_vsock_addr)) => {
					unreachable!("Connected to something that wasn't a vsock")
				},
			};

			info!(
				cid = addr.cid(),
				port = &addr.port(),
				%socket_path,
				"Forwarding connection from host to unix"
			);

			// Spawn a task to handle each connection
			let socket_path = socket_path.to_owned();
			tokio::task::spawn(async move {
				let _ = handle_forward_connection(stream, socket_path)
					.await
					.map_err(|e| error!(err = ?e, "Error forwarding connection"));
			});
		}
	}

	#[instrument(level = "INFO", skip_all, fields(socket_path))]
	async fn handle_forward_connection(
		mut vsock_stream: VsockStream,
		socket_path: Utf8PathBuf,
	) -> Result<()> {
		// Connect to the socket.
		let mut unix_stream = UnixStream::connect(socket_path)
			.await
			.context("failed to connect to unix socket")?;

		// Plug the two streams into each other.
		// TODO: use `splice(2)` here for zero-copy
		tokio::io::copy_bidirectional(&mut vsock_stream, &mut unix_stream)
			.await
			.context("failed to forward data")?;

		Ok(())
	}

	#[derive(Debug, Clone)]
	pub struct ForwardArg {
		port: u32,
		socket_path: Utf8PathBuf,
	}

	impl std::str::FromStr for ForwardArg {
		type Err = anyhow::Error;
		fn from_str(source: &str) -> Result<ForwardArg> {
			let split: Vec<&str> = source.split(":").collect();
			if let &[port_str, path_str] = split.as_slice() {
				let port: u32 = port_str
					.parse()
					.context("invalid port part of forward argument")?;
				let socket_path = Utf8PathBuf::from(path_str);

				Ok(ForwardArg { port, socket_path })
			} else {
				bail!("invalid forward argument: expected <vsock_port>:<unix_socket_path>");
			}
		}
	}
}

#[cfg(not(target_os = "linux"))]
fn main() -> anyhow::Result<()> {
	Err(anyhow::Error::msg(
		"Tangram guest-agent is only supported on Linux platforms",
	))
}

#[cfg(target_os = "linux")]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
	linux::main().await
}
