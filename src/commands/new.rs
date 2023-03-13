use crate::Cli;
use tangram::{
	error::{Context, Result},
	util::fs,
};

/// Create a new package.
#[derive(clap::Args)]
pub struct Args {
	#[arg(long)]
	pub name: Option<String>,

	#[arg(long)]
	pub version: Option<String>,

	pub path: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_new(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Create a directory at the path.
		tokio::fs::create_dir_all(&path).await.with_context(|| {
			let path = path.display();
			format!(r#"Failed to create the directory at "{path}"."#)
		})?;

		// Init.
		self.command_init(super::init::Args {
			name: args.name,
			path: args.path,
			version: args.version,
		})
		.await?;

		Ok(())
	}
}
