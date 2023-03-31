use crate::{
	error::{return_error, Result},
	Cli,
};

/// Check for outdated dependencies.
#[derive(Debug, clap::Args)]
pub struct Args {}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_outdated(&self, _args: Args) -> Result<()> {
		return_error!("This command is not yet implemented.");
	}
}
