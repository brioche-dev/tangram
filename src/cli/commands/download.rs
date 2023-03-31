use crate::{
	error::{Error, Result},
	Cli,
};
use tangram::operation::{Download, Operation};
use url::Url;

/// Run a download operation.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(long)]
	pub unpack: bool,

	/// The URL to download from.
	pub url: Url,
}

impl Cli {
	pub async fn command_download(&self, args: Args) -> Result<()> {
		// Run the operation.
		let operation = Operation::Download(Download {
			url: args.url,
			unpack: args.unpack,
			checksum: None,
			is_unsafe: true,
		});
		let output = operation.run(&self.tg).await?;

		// Print the output.
		let output = serde_json::to_string_pretty(&output).map_err(Error::other)?;
		println!("{output}");

		Ok(())
	}
}
