use crate::{
	error::{Result, WrapErr},
	Cli,
};
use futures::FutureExt;
use tangram::util::fs;

/// Manage autoenv paths.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
	/// List autoenv paths.
	List(ListArgs),

	/// Add an autoenv path.
	Add(AddArgs),

	/// Remove an autoenv path.
	Remove(RemoveArgs),
}

/// List autoenv paths.
#[derive(Debug, clap::Args)]
pub struct ListArgs {}

/// Add an autoenv path.
#[derive(Debug, clap::Args)]
pub struct AddArgs {
	path: Option<fs::PathBuf>,
}

/// Remove an autoenv path.
#[derive(Debug, clap::Args)]
pub struct RemoveArgs {
	path: Option<fs::PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct HookArgs {
	shell: String,
}

impl Cli {
	pub async fn command_autoenv(&self, args: Args) -> Result<()> {
		match args.command {
			Command::List(args) => self.command_autoenv_list(args).boxed(),
			Command::Add(args) => self.command_autoenv_add(args).boxed(),
			Command::Remove(args) => self.command_autoenv_remove(args).boxed(),
		}
		.await?;
		Ok(())
	}

	async fn command_autoenv_list(&self, _args: ListArgs) -> Result<()> {
		// Read the config.
		let config = self.read_config().await?.unwrap_or_default();

		// List the autoenvs.
		let autoenvs = config.autoenvs.unwrap_or_default();

		if autoenvs.is_empty() {
			eprintln!("There are no autoenvs.");
		}

		for path in autoenvs {
			let path = path.display();
			println!("{path}");
		}

		Ok(())
	}

	async fn command_autoenv_add(&self, args: AddArgs) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().wrap_err("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}
		let path = tokio::fs::canonicalize(&path)
			.await
			.wrap_err("Failed to canonicalize the path.")?;

		// Read the config.
		let mut config = self.read_config().await?.unwrap_or_default();

		// Add the autoenv.
		let mut autoenvs = config.autoenvs.unwrap_or_default();
		autoenvs.push(path);
		config.autoenvs = Some(autoenvs);

		// Write the config.
		self.write_config(&config).await?;

		Ok(())
	}

	async fn command_autoenv_remove(&self, args: RemoveArgs) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().wrap_err("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}
		let path = tokio::fs::canonicalize(&path)
			.await
			.wrap_err("Failed to canonicalize the path.")?;

		// Read the config.
		let mut config = self.read_config().await?.unwrap_or_default();

		// Remove the autoenv.
		if let Some(mut autoenvs) = config.autoenvs {
			if let Some(index) = autoenvs.iter().position(|p| *p == path) {
				autoenvs.remove(index);
			}
			config.autoenvs = Some(autoenvs);
		}

		// Write the config.
		self.write_config(&config).await?;

		Ok(())
	}
}
