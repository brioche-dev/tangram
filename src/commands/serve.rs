use crate::Cli;
use anyhow::{Context, Result};
use std::net::{IpAddr, SocketAddr};

/// Run a server.
#[derive(clap::Args)]
pub struct Args {
	#[arg(long, default_value = "0.0.0.0")]
	host: IpAddr,
	#[arg(long, default_value = "8080")]
	port: u16,
}

impl Cli {
	pub async fn command_serve(&self, args: Args) -> Result<()> {
		// Serve!
		let addr = SocketAddr::new(args.host, args.port);
		self.tg.serve(addr).await.context("Failed to serve.")?;

		Ok(())
	}
}
