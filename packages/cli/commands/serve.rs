use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::net::{IpAddr, SocketAddr};
use tangram_core::server::Server;

#[derive(Parser)]
pub struct Args {
	#[arg(long, default_value = "0.0.0.0")]
	host: IpAddr,
	#[arg(long, default_value = "8080")]
	port: u16,
}

impl Cli {
	pub(crate) async fn command_serve(&self, args: Args) -> Result<()> {
		// Create the builder.
		let builder = crate::builder().await?;

		// Create the server.
		let server = Server::new(builder);

		// Serve!
		let addr = SocketAddr::new(args.host, args.port);
		server.serve(addr).await.context("Failed to serve.")?;

		Ok(())
	}
}
