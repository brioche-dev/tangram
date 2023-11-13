use crate::Cli;
use std::path::PathBuf;
use tangram_client::package::Builder;
use tangram_error::Result;

/// Update a package's dependencies.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub path: PathBuf,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_update(&self, args: Args) -> Result<()> {
		let client = self.client().await?;
		let client = client.as_ref();
		let mut builder = tangram_package::Builder::new(&args.path);
		builder.update(client, None).await?;
		Ok(())
	}
}
