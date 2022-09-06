use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use std::net::{IpAddr, SocketAddr};
use tangram::server::Server;

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = "0.0.0.0")]
	host: IpAddr,
	#[clap(long, default_value = "8080")]
	port: u16,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read()
		.await
		.context("Failed to read the server config.")?;

	// Create the server.
	let server = Server::new(config.server)
		.await
		.context("Failed to create the server.")?;

	// Serve!
	let addr = SocketAddr::new(args.host, args.port);
	server.serve_tcp(addr).await.context("Failed to serve.")?;

	Ok(())
}
