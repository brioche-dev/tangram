use crate::{return_error, Cli, Result};
use bytes::Bytes;
use futures::TryStreamExt;
use tangram_client as tg;
use tokio::io::AsyncBufReadExt;

/// Get the log for a build.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The ID of the build or target to get the log for.
	pub id: tg::Id,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_log(&self, args: Args) -> Result<()> {
		let build = if let Ok(id) = tg::build::Id::try_from(args.id) {
			tg::Build::with_id(id)
		} else if let Ok(id) = tg::target::Id::try_from(args.id) {
			tg::Target::with_id(id).build(self.client.as_ref()).await?
		} else {
			return_error!("The ID must be a target or build ID.");
		};

		// Write the log to stdout.
		let log = build.log(self.client.as_ref()).await?;
		let log =
			tokio_util::io::StreamReader::new(log.map_ok(Bytes::from).map_err(|error| {
				std::io::Error::new(std::io::ErrorKind::Other, error.to_string())
			}));
		let mut lines = log.lines();
		while let Some(line) = lines.next_line().await? {
			println!("{line}");
		}

		Ok(())
	}
}
