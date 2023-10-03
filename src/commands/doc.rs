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
		todo!()

		// // Get the package.
		// let package = tg::Package::with_specifier(&self.client, args.package)
		// 	.await
		// 	.wrap_err("Failed to get the package.")?;

		// // Create the language service.
		// let language_service = tg::language::Service::new(self.client.clone(), None);

		// // Get the docs.
		// let docs = package.docs(&self.client, &language_service).await?;

		// // Render the docs to JSON.
		// let json = serde_json::to_string_pretty(&docs).map_err(Error::with_error)?;

		// // Print the docs.
		// println!("{json}");

		// Ok(())
	}
}
