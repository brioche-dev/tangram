use super::PackageArgs;
use crate::Cli;
use tangram_client as tg;
use tangram_lsp::ROOT_MODULE_FILE_NAME;
use tg::{Result, WrapErr};

/// Print the docs for a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: tangram_package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(short, long, default_value = "false")]
	pub runtime: bool,
}

impl Cli {
	pub async fn command_doc(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();

		// Create the language server.
		let server =
			tangram_lsp::Server::new(client.downgrade_box(), tokio::runtime::Handle::current());

		let (module, path) = if args.runtime {
			// Create the module.
			let module = tangram_lsp::Module::Library(tangram_lsp::module::Library {
				path: "tangram.d.ts".parse().unwrap(),
			});
			(module, "tangarm.d.ts")
		} else {
			// Create the package.
			let (package, lock) = tangram_package::new(client, &args.package)
				.await
				.wrap_err("Failed to create the package.")?;
			// Create the module.
			let module = tangram_lsp::Module::Normal(tangram_lsp::module::Normal {
				package: package.id(client).await?.clone(),
				path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
				lock: lock.id(client).await?.clone(),
			});
			(module, ROOT_MODULE_FILE_NAME)
		};

		// Get the docs.
		let docs = server.docs(&module).await?;
		// Render the docs to JSON.
		let docs = serde_json::to_string_pretty(&serde_json::json!({
			path.to_owned(): docs,
		}))
		.wrap_err("Failed to serialize the docs.")?;

		// Print the docs.
		println!("{docs}");

		Ok(())
	}
}
