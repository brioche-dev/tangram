use crate::Cli;
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
pub struct Args {}

impl Cli {
	#[allow(clippy::unused_async)]
	pub(crate) async fn command_update(&self, _args: Args) -> Result<()> {
		todo!()
	}
}
