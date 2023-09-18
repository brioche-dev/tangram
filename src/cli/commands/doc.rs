use super::PackageArgs;
use crate::{
	error::{Error, Result, WrapErr},
	Cli,
};

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
		let client = &self.client;

		// Get the package.
		let package = tg::Package::with_specifier(client, args.package)
			.await
			.wrap_err("Failed to get the package.")?;

		// Get the root module.
		let root_module = package.root_module();

		// Get the doc.
		let doc = root_module.docs(client).await?;

		// Render the doc to JSON.
		let json = serde_json::to_string_pretty(&doc).map_err(Error::other)?;

		// Print the doc.
		println!("{json}");

		Ok(())
	}
}
