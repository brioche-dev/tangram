use crate::{Cli, Result, WrapErr};
use std::{
	net::{IpAddr, SocketAddr},
	path::PathBuf,
};

/// Run a server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The host to bind the server to.
	#[arg(long, default_value = "0.0.0.0")]
	pub host: IpAddr,

	/// The port to bind the server to.
	#[arg(long, default_value = "8476")]
	pub port: u16,

	/// The path where Tangram should store its data. The default is `$HOME/.tangram`.
	#[arg(long, env = "TANGRAM_PATH")]
	pub path: Option<PathBuf>,
}

impl Cli {
	pub async fn command_serve(&self, args: Args) -> Result<()> {
		// Get the path.
		let path = if let Some(path) = args.path.clone() {
			path
		} else {
			tg::util::dirs::home_directory_path()
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
			.and_then(|config| config.parent_url.as_ref())
			.cloned();

		// Get the origin token.
		let parent_token = credentials.map(|credentials| credentials.token);

		// Create the options.
		let options = tg::server::Options {
			parent_token,
			parent_url,
		};

		// Create the server.
		let server = tg::Server::new(path, options).await?;

		// Serve.
		let addr = SocketAddr::new(args.host, args.port);
		server
			.serve(addr)
			.await
			.wrap_err("Failed to run the server.")?;

		Ok(())
	}
}
