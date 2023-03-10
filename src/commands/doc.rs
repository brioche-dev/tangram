use crate::Cli;
use tangram::{error::Result, module, package};

/// Print the docs for a package.
#[derive(clap::Args)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	package_specifier: package::Specifier,
}

impl Cli {
	pub async fn command_doc(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self
			.tg
			.resolve_package(&args.package_specifier, None)
			.await?;

		// Get the package instance hash.
		let package_instance_hash = self
			.tg
			.create_package_instance(&package_identifier, args.locked)
			.await?;

		// Create the root module identifier.
		let root_module_identifier =
			module::Identifier::for_root_module_in_package_instance(package_instance_hash);

		// Get the doc.
		let doc = self.tg.doc(root_module_identifier).await?;

		// Render the doc to JSON.
		let string = serde_json::to_string_pretty(&doc)?;

		// Print the doc.
		println!("{string}");

		Ok(())
	}
}
