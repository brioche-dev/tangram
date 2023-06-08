use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::net::{IpAddr, SocketAddr};

/// Run a server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(long, default_value = "0.0.0.0")]
	pub host: IpAddr,

	#[arg(long, default_value = "8476")]
	pub port: u16,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_serve(&self, args: Args) -> Result<()> {
		// Create the server.
		let addr = SocketAddr::new(args.host, args.port);
		let server = tangram::server::Server::new(self.tg.clone());

		// Run the server.
		server
			.serve(addr)
			.await
			.wrap_err("Failed to run the server.")?;

		Ok(())
	}
}
