use crate::{
	error::{return_error, Result},
	Cli,
};
use tangram::checksum;

/// Compute a checksum.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The checksum algorithm to use.
	#[arg(short, long)]
	pub algorithm: checksum::Algorithm,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_checksum(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
