use crate::{util::dirs::home_directory_path, Cli, API_URL};
use std::path::PathBuf;
use tangram_client as tg;
use tangram_util::addr::Addr;
use tg::{Result, WrapErr};

/// Run a server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
	/// The host to bind the server to.
	#[arg(long, value_parser = parse_addr)]
	pub addr: Option<Addr>,

	/// The path where Tangram should store its data. The default is `$HOME/.tangram`.
	#[arg(long, env = "TANGRAM_PATH")]
	pub path: Option<PathBuf>,

	#[arg(long, default_value = "start")]
	pub command: Action,

	#[arg(long)]
	pub daemonize: bool,
}

fn parse_addr(s: &str) -> Result<Addr, String> {
	s.parse().map_err(|_| "Failed to parse address.".to_owned())
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum Action {
	Start,
	Stop,
	Ping,
}

impl Cli {
	pub async fn command_serve(&self, args: Args) -> Result<()> {
		// Get the path.
		let path = if let Some(path) = args.path.clone() {
			path
		} else {
			home_directory_path()
				.wrap_err("Failed to find the user home directory.")?
				.join(".tangram")
		};

		// Get the addr.
		let addr = args.addr.unwrap_or(Addr::Socket(path.join("socket")));

		// Ping or stop, if necessary.
		match args.command {
			Action::Ping => {
				let client = tg::Remote::new(addr.clone(), None)
					.await
					.wrap_err("Failed to create client.")?;
				client.ping().await?;
			},
			Action::Stop => {
				let client = tg::Remote::new(addr.clone(), None)
					.await
					.wrap_err("Failed to create client.")?;
				let _ = client.ping().await;
				return Ok(());
			},
			Action::Start => (),
		}

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
		let _parent = tangram_client::remote::Remote::new(Addr::Inet(parent_url), parent_token);

		// Create the server.
		let server = tangram_server::Server::new(path, None).await?;

		// Serve.
		let server_task = tokio::task::spawn(async move {
			server
				.serve(addr)
				.await
				.wrap_err("Failed to run the server.")
		});

		server_task.await.wrap_err("Failed to join server task")??;
		Ok(())
	}
}
