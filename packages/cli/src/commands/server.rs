use crate::Cli;
use tangram_client as tg;
use tg::Result;

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

	/// Restart the server.
	Restart,

	/// Ping the server.
	Ping,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_server(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Start => todo!(),
			Command::Stop => todo!(),
			Command::Restart => todo!(),
			Command::Ping => todo!(),
		}
	}
}
