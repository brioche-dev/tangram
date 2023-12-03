use crate::Cli;
use std::path::PathBuf;
use tangram_error::{Result, WrapErr};

/// Format the files in a package.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(default_value = ".")]
	pub path: PathBuf,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_fmt(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();

		// Create the language server.
		let server = tangram_language::Server::new(client, tokio::runtime::Handle::current());

		let path = args.path.join(tangram_package::ROOT_MODULE_FILE_NAME);
		let text = tokio::fs::read_to_string(&path)
			.await
			.wrap_err("Failed to read the file.")?;
		let text = server.format(text).await?;
		tokio::fs::write(&path, text)
			.await
			.wrap_err("Failed to write the file.")?;

		Ok(())
	}
}
