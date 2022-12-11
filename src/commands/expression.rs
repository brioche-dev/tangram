use crate::{hash::Hash, Cli};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(long_about = "Manage expressions.")]
pub struct Args {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	Get(GetArgs),
}

#[derive(Parser, Debug)]
#[command(long_about = "Get an expression.")]
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
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// Get the expression.
		let expression = cli.get_expression_local(args.expression)?;

		// Serialize the expression.
		let json = serde_json::to_string_pretty(&expression)?;

		// Print it.
		println!("{json}");

		Ok(())
	}
}
