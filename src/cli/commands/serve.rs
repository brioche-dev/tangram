use crate::{
	error::{Result, WrapErr},
	Cli,
};
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

	/// If set, temp files and directories will persist after the process exits.
	#[arg(long, env = "TANGRAM_PRESERVE_TEMPS")]
	pub preserve_temps: Option<bool>,

	/// If set, child processes will be run un-sandboxed.
	#[arg(long, env = "TANGRAM_SANDBOX_ENABLED")]
	pub sandbox_enabled: Option<bool>,
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

		// Get the preserve temps configuration.
		let preserve_temps = args
			.preserve_temps
			.or(config.as_ref().and_then(|config| config.preserve_temps))
			.unwrap_or(false);

		// Get the sandbox configuration.
		let sandbox_enabled = args
			.sandbox_enabled
			.or(config.as_ref().and_then(|config| config.sandbox_enabled))
			.unwrap_or(true);

		// Read the credentials.
		let credentials = Self::read_credentials().await?;

		// Get the origin URL.
		let origin_url = config
			.as_ref()
			.and_then(|config| config.origin_url.as_ref())
			.cloned();

		// Get the origin token.
		let origin_token = credentials.map(|credentials| credentials.token);

		// Create the options.
		let options = tg::server::Options {
			parent_token: origin_token,
			parent_url: origin_url,
			preserve_temps,
			sandbox_enabled,
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
