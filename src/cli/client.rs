use crate::config::Config;
use anyhow::{Context, Result};
use tangram::client::Client;

pub async fn new() -> Result<Client> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(tangram::client::config::Config {
		transport: tangram::client::config::Transport::InProcess {
			server: tangram::server::config::Config {
				path: config.path,
				peers: config.peers,
			},
		},
	})
	.await
	.context("Failed to create the client.")?;

	Ok(client)
}
