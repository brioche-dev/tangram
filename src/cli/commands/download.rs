use crate::{
	error::{Error, Result},
	Cli,
};
use tangram::operation::{Download, Operation};
use url::Url;

/// Run a download operation.
#[derive(clap::Args)]
pub struct Args {
	/// The URL to download from.
	url: Url,

	unpack: bool,
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
