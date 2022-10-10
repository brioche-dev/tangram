use crate::Cli;
use anyhow::Result;
use clap::Parser;
use tangram_core::hash::Hash;

#[derive(Parser)]
pub struct Args {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	Get(GetArgs),
}

#[derive(Parser, Debug)]
pub struct GetArgs {
	expression: Hash,
}

impl Cli {
	pub(crate) async fn command_expression(&self, args: Args) -> Result<()> {
		match args.subcommand {
			Subcommand::Get(args) => self.command_expression_get(args),
		}
		.await?;
		Ok(())
	}

	pub async fn command_expression_get(&self, args: GetArgs) -> Result<()> {
		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Get the expression.
		let expression = builder.get_expression(args.expression)?;

		// Serialize the expression.
		let json = serde_json::to_string_pretty(&expression)?;

		// Print it.
		println!("{json}");

		Ok(())
	}
}
