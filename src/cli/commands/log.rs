use crate::{error::Result, Cli};
use tangram::operation;

/// Get the log for an operation.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The hash of the operation to get logs from.
	pub operation_hash: operation::Hash,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn command_log(&self, _args: Args) -> Result<()> {
		unimplemented!()

		// // Get the log reader.
		// let mut reader = self.tg.get_log_reader(args.operation_hash).await?;

		// // Copy the log to stdout.
		// let mut stdout = tokio::io::stdout();
		// tokio::io::copy(&mut reader, &mut stdout)
		// 	.await
		// 	.wrap_err("Failed to write the log to stdout.")?;

		// Ok(())
	}
}
