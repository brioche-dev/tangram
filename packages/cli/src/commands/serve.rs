use crate::{util::dirs::home_directory_path, Cli, API_URL};
use std::{
	net::{IpAddr, SocketAddr},
	path::PathBuf,
};
use tangram_client as tg;
use tg::{Result, WrapErr};

/// Run a server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The host to bind the server to.
	#[command(subcommand)]
	pub addr: Addr,

	/// The path where Tangram should store its data. The default is `$HOME/.tangram`.
	#[arg(long, env = "TANGRAM_PATH")]
	pub path: Option<PathBuf>,

	#[arg(long, default_value = "start")]
	pub command: Command,

	#[arg(long)]
	pub daemonize: bool,
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum Command {
	Start,
	Stop,
	Ping,
}

#[derive(Debug, clap::Subcommand)]
pub enum Addr {
	Inet {
		#[arg(long, default_value = "0.0.0.0")]
		host: IpAddr,

		#[arg(long, default_value = "8476")]
		port: u16,
	},
	Socket {
		#[arg(long)]
		path: PathBuf,
	},
}

impl Cli {
	pub async fn command_serve(&self, args: Args) -> Result<()> {
		let addr = match args.command {
			Command::Ping => {
				let addr = match args.addr {
					Addr::Inet { host, port } => {
						tg::hyper::Addr::Inet(format!("http://{host}:{port}").parse().unwrap())
					},
					Addr::Socket { path } => tg::hyper::Addr::Socket(path),
				};
				let client = tg::hyper::Hyper::new(addr, None)
					.await
					.wrap_err("Failed to create client.")?;
				client.ping().await?;
				println!("Server online.");
				return Ok(());
			},
			Command::Stop => {
				let addr = match args.addr {
					Addr::Inet { host, port } => {
						tg::hyper::Addr::Inet(format!("http://{host}:{port}").parse().unwrap())
					},
					Addr::Socket { path } => tg::hyper::Addr::Socket(path),
				};
				let client = tg::hyper::Hyper::new(addr, None)
					.await
					.wrap_err("Failed to create client.")?;
				let _ = client.stop().await;
				return Ok(());
			},
			Command::Start => match args.addr {
				Addr::Inet { host, port } => {
					tangram_server::Addr::Inet(SocketAddr::new(host, port))
				},
				Addr::Socket { path } => tangram_server::Addr::Socket(path),
			},
		};

		// Daemonize.
		if args.daemonize {
			daemonize().wrap_err("Failed to daemonize the server.")?;
		}

		// Get the path.
		let path = if let Some(path) = args.path.clone() {
			path
		} else {
			home_directory_path()
				.wrap_err("Failed to find the user home directory.")?
				.join(".tangram")
		};

		// Read the config.
		let config = Self::read_config().await?;

		// Read the credentials.
		let credentials = Self::read_credentials().await?;

		// Get the parent URL.
		let parent_url = config
			.as_ref()
			.and_then(|config| config.parent_url.as_ref().cloned())
			.unwrap_or_else(|| API_URL.parse().unwrap());

		// Get the parent token.
		let parent_token = credentials.map(|credentials| credentials.token);

		// Create the parent.
		let _parent = tangram_client::hyper::Hyper::new(
			tangram_client::hyper::Addr::Inet(parent_url),
			parent_token,
		);

		// Create the server.
		let server = tangram_server::Server::new(path, None).await?;

		// Serve.
		server
			.serve(addr)
			.await
			.wrap_err("Failed to run the server.")?;

		Ok(())
	}
}

extern "C" {
	fn daemon(nochdir: i32, noclose: i32) -> i32;
}

fn daemonize() -> std::io::Result<()> {
	unsafe {
		let err = daemon(1, 1);
		if err != 0 {
			return Err(std::io::Error::last_os_error());
		}
	}
	Ok(())
}
