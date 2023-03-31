use super::PackageArgs;
use crate::{
	error::{Error, Result},
	Cli,
};
use tangram::{module, package};

/// Print the docs for a package.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,
}

impl Cli {
	pub async fn command_doc(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self.tg.resolve_package(&args.package, None).await?;

		// Create the package instance.
		let package_instance_hash = self
			.tg
			.clone()
			.create_package_instance(&package_identifier, args.package_args.locked)
			.await?;

		// Create the root module identifier.
		let root_module_identifier =
			module::Identifier::for_root_module_in_package_instance(package_instance_hash);

		// Get the doc.
		let doc = self.tg.doc(root_module_identifier).await?;

		// Render the doc to JSON.
		let string = serde_json::to_string_pretty(&doc).map_err(Error::other)?;

		// Print the doc.
		println!("{string}");

		Ok(())
	}
}
