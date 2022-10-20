use crate::Cli;
use anyhow::Result;
use clap::Parser;
use tangram_core::{js, specifier::Specifier};

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	pub(crate) async fn command_check(&self, args: Args) -> Result<()> {
		let compiler = js::Compiler::new(self.builder.clone());

		let package_hash = self
			.package_hash_for_specifier(&args.specifier, args.locked)
			.await?;

		let diagnostics = compiler.check(package_hash).await?;

		for diagnostic in diagnostics {
			println!("{diagnostic}");
		}

		Ok(())
	}
}
