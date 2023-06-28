use super::PackageArgs;
use crate::{
	error::{Error, Result, WrapErr},
	Cli,
};
use tangram::package::{self, Package};

/// Print the docs for a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_doc(&self, args: Args) -> Result<()> {
		// Get the package.
		let package = Package::with_specifier(&self.tg, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Get the root module.
		let root_module = package.root_module();

		// Get the doc.
		let doc = root_module.docs(&self.tg).await?;

		// Render the doc to JSON.
		let json = serde_json::to_string_pretty(&doc).map_err(Error::other)?;

		// Print the doc.
		println!("{json}");

		Ok(())
	}
}
