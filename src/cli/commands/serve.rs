use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::{
	net::{IpAddr, SocketAddr},
	sync::Arc,
};

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
		// Create the server.
		let addr = SocketAddr::new(args.host, args.port);
		let server = tangram::server::Server::new(Arc::clone(&self.tg));

		// Run the server.
		server
			.serve(addr)
			.await
			.wrap_err("Failed to run the server.")?;

		Ok(())
	}
}
