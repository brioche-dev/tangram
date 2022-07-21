use crate::dirs::home_dir;
use anyhow::{anyhow, Result};
use tangram::client::Client;

pub async fn new() -> Result<Client> {
	let path = home_dir()
		.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
		.join(".tangram");
	let client = Client::new_in_process(path).await?;
	// let client = tangram::Client::new_tcp("http://localhost:8080".parse().unwrap()).await?;
	Ok(client)
}
