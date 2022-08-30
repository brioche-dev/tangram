use crate::dirs::home_directory_path;
use anyhow::{anyhow, Result};
use clap::Parser;
use std::{
	net::{IpAddr, SocketAddr},
	path::PathBuf,
	sync::Arc,
};
use tangram::server::Server;

#[derive(Parser)]
pub struct Args {
	#[clap(long)]
	path: Option<PathBuf>,
	#[clap(long, default_value = "0.0.0.0")]
	host: IpAddr,
	#[clap(long, default_value = "8080")]
	port: u16,
}

pub async fn run(args: Args) -> Result<()> {
	// Get the server path.
	let path = home_directory_path()
		.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
		.join(".tangram");

	// Create the server.
	let server = Arc::new(Server::new(path).await?);

	// Serve!
	let addr = SocketAddr::new(args.host, args.port);
	server.serve_tcp(addr).await?;

	Ok(())
}
