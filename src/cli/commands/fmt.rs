use crate::{error::Result, Cli};
use tangram::package;

/// Format the files in a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(default_value = ".")]
	pub package: package::Specifier,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_fmt(&self, _args: Args) -> Result<()> {
		unimplemented!()

		// // Format each module.
		// for module in package.modules(&self.tg).await? {
		// 	// Get the module's path.
		// 	let path = match &module.source {
		// 		module::Source::Package(Package::Path(package_path)) => {
		// 			package_path.join(&module.path.to_string())
		// 		},
		// 		_ => unreachable!(),
		// 	};

		// 	// Get the module's text.
		// 	let text = module.load(&self.tg).await?;

		// 	// Format the text.
		// 	let text = Module::format(&self.tg, text).await?;

		// 	// Save the formatted text.
		// 	tokio::fs::write(path, text).await?;
		// }

		// Ok(())
	}
}
