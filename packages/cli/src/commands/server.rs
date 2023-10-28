use crate::Cli;
use tangram_client as tg;
use tg::{Result, WrapErr};

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

	/// Stop the server.
	Stop,

	/// Ping the server.
	Ping,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_server(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Start => {
				tokio::process::Command::new(
					std::env::current_exe()
						.wrap_err("Failed to get the current executable path.")?,
				)
				.arg("serve")
				.spawn()
				.wrap_err("Failed to spawn the server.")?;
			},
			Command::Stop => {
				let client = self.client().await?;
				let client = client.as_ref();
				client.stop().await?;
			},
			Command::Ping => {
				let client = self.client().await?;
				let client = client.as_ref();
				client.ping().await?;
			},
		}
		Ok(())
	}
}
