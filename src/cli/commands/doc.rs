use super::PackageArgs;
use crate::{Cli, Error, Result, WrapErr};

/// Print the docs for a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: tg::package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_doc(&self, args: Args) -> Result<()> {
		// Get the package.
		let package = tg::Package::with_specifier(&self.client, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Get the doc.
		let doc = package.doc(&self.client).await?;

		// Render the doc to JSON.
		let json = serde_json::to_string_pretty(&doc).map_err(Error::other)?;

		// Print the doc.
		println!("{json}");

		Ok(())
	}
}
