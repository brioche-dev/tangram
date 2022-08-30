use crate::config::{self, Config};
use anyhow::Result;
use tangram::{client::Client, server::Server};

pub async fn new() -> Result<Client> {
	// Read the config.
	let config = Config::read().await?;

	// Create the client.
	let client = match config.transport {
		config::Transport::InProcess { path } => {
			let server = Server::new(&path).await?;
			Client::new_in_process(server)
		},
		config::Transport::Unix { path } => Client::new_unix(path),
		config::Transport::Tcp { url } => Client::new_tcp(url),
	};

	Ok(client)
}
