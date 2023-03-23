use crate::{error::Result, Cli};
use tangram::checksum;

/// Compute a checksum.
#[derive(clap::Args)]
pub struct Args {
	/// The checksum algorithm to use.
	pub algorithm: checksum::Algorithm,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_checksum(&self, _args: Args) -> Result<()> {
		Ok(())
	}
}
