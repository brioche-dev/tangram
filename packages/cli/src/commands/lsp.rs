use std::path::PathBuf;

use crate::Cli;
use tangram_client::package::Builder;
use tangram_error::{Result, WrapErr};
use tokio::io::AsyncReadExt;

/// Run the language server.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(long)]
	pub workspace_root: Option<PathBuf>,
}

impl Cli {
	pub async fn command_lsp(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();

		// Create the package builder if we have a workspace root.
		let package_builder: Option<Box<dyn tangram_client::package::Builder>> =
			match args.workspace_root {
				Some(workspace_root) => {
					let mut builder = tangram_package::Builder::new(&workspace_root);
					let lockfile_path = workspace_root.join(tangram_package::LOCKFILE_FILE_NAME);
					let lockfile = if lockfile_path.exists() {
						let mut file = tokio::fs::File::open(&lockfile_path)
							.await
							.wrap_err("Failed to open lockfile.")?;
						let mut contents = Vec::new();
						file.read_to_end(&mut contents)
							.await
							.wrap_err("Failed to read lockfile contents.")?;
						let lockfile = serde_json::from_slice(&contents)
							.wrap_err("Failed to deserialize lockfile.")?;
						Some(lockfile)
					} else {
						None
					};
					builder.update(client, lockfile).await?;
					Some(Box::new(builder))
				},
				None => None,
			};

		// Create the language server.
		let server =
			tangram_lsp::Server::new(client, tokio::runtime::Handle::current(), package_builder);

		// Run the language server.
		server.serve().await?;

		Ok(())
	}
}
