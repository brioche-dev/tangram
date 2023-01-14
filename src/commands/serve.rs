use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use std::net::{IpAddr, SocketAddr};

#[derive(Parser)]
#[command(about = "Run a server.")]
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
		self.serve(addr).await.context("Failed to serve.")?;

		Ok(())
	}
}
