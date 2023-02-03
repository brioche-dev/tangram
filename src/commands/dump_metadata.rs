use crate::{package_specifier::PackageSpecifier, Cli};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Dump the metadata of a package.")]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: PackageSpecifier,
}

impl Cli {
	pub async fn command_dump_metadata(&self, args: Args) -> Result<()> {
		// Get the entrypoint module identifier.
		let module_identifier = self
			.entrypoint_module_identifier_for_specifier(&args.specifier)
			.await?;

		let export_metadata = self.get_metadata(&module_identifier).await?;

		let export_metadata_json = serde_json::to_string_pretty(&export_metadata)?;
		println!("{export_metadata_json}");

		Ok(())
	}
}
