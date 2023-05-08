use crate::{
	error::{Error, Result},
	Cli,
};
use tangram::operation::Download;
use url::Url;

/// Run a download operation.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(long)]
	pub unpack: bool,

	/// The URL to download from.
	pub url: Url,
}

impl Cli {
	pub async fn command_download(&self, args: Args) -> Result<()> {
		// Run the operation.
		let download = Download::builder(args.url)
			.unpack(args.unpack)
			.unsafe_(true)
			.build(&self.tg)
			.await?;
		let output = download.run(&self.tg).await?;

		// Print the output.
		let output = serde_json::to_string_pretty(&output).map_err(Error::other)?;
		println!("{output}");

		Ok(())
	}
}
