use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::path::PathBuf;
use tangram::{artifact::Artifact, block::Block, id::Id};

/// Check out an artifact.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	/// The ID of the artifact to check out.
	pub id: Id,

	/// The path to check out the artifact to.
	pub path: Option<PathBuf>,
}

impl Cli {
	pub async fn command_checkout(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path = std::env::current_dir().wrap_err("Failed to get the working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		} else {
			path.push(args.id.to_string());
		};

		// Get the artifact.
		let block = Block::with_id(args.id);
		let artifact = Artifact::get(&self.tg, block)
			.await
			.wrap_err("Failed to get the artifact.")?;

		// Check out the artifact.
		artifact
			.check_out(&self.tg, &path)
			.await
			.wrap_err("Failed to check out the artifact.")?;

		Ok(())
	}
}
