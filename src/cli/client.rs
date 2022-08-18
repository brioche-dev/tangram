use crate::dirs::home_dir;
use anyhow::{anyhow, Result};
use tangram::{client::Client, server::Server};

pub async fn new() -> Result<Client> {
	let path = home_dir()
		.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
		.join(".tangram");
	let server = Server::new(&path).await?;
	let client = Client::new_in_process(server);
	Ok(client)
}
