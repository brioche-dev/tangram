use crate::Cli;
use tangram_client as tg;
use tangram_util::net::Addr;
use tg::{Client, Result};

/// Manage the server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
	/// Start the server.
	Start,

	/// Get the server's status.
	Status,

	/// Stop the server.
	Stop,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_server(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Start => {
				self.start_server().await?;
			},
			Command::Status => {
				let addr = Addr::Unix(self.path.join("socket"));
				let client = tangram_client::remote::Builder::new(addr).build();
				let status = client.status().await?;
				let status = serde_json::to_string_pretty(&status).unwrap();
				println!("{status}");
			},
			Command::Stop => {
				let addr = Addr::Unix(self.path.join("socket"));
				let client = tangram_client::remote::Builder::new(addr).build();
				client.stop().await?;
			},
		}
		Ok(())
	}
}
