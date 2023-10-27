use crate::{util::dirs::home_directory_path, Cli, API_URL};
use std::path::PathBuf;
use tangram_client as tg;
use tangram_util::net::Addr;
use tg::{Result, WrapErr};

/// Run a server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
#[allow(clippy::struct_excessive_bools)]
pub struct Args {
	/// The address to bind the server to.
	#[arg(long, value_parser = parse_addr)]
	pub address: Option<Addr>,

	/// The path where Tangram should store its data. The default is `$HOME/.tangram`.
	#[arg(long, env = "TANGRAM_PATH")]
	pub path: Option<PathBuf>,
}

fn parse_addr(s: &str) -> Result<Addr, String> {
	s.parse().map_err(|_| "Failed to parse address.".to_owned())
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
		let addr = args.address.unwrap_or(Addr::Unix(path.join("socket")));

		// Read the config.
		let config = Self::read_config().await?;

		// Read the credentials.
		let credentials = Self::read_credentials().await?;

		// Create the parent.
		let parent_url = config
			.as_ref()
			.and_then(|config| config.parent_url.as_ref().cloned())
			.unwrap_or_else(|| API_URL.parse().unwrap());
		let parent_addr = parent_url
			.authority()
			.parse()
			.wrap_err("Invalid parent URL.")?;
		let parent_tls = parent_url.scheme() == "https";
		let parent_token = credentials.map(|credentials| credentials.token);
		let parent = tangram_client::remote::Builder::new(parent_addr)
			.tls(parent_tls)
			.token(parent_token)
			.build()
			.await?;
		let _parent = Box::new(parent);

		// Create the server.
		let server = tangram_server::Server::new(path, None)
			.await
			.wrap_err("Failed to create the server.")?;

		// Serve.
		server
			.serve(addr)
			.await
			.wrap_err("Failed to run the server.")?;

		Ok(())
	}
}
