use crate::Cli;
use futures::FutureExt;
use itertools::Itertools;
use std::path::PathBuf;
use tangram_client as tg;
use tg::{Result, WrapErr};

/// Manage autoenv paths.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
	/// Add an autoenv path.
	Add(AddArgs),

	/// Get the autoenv path for a path.
	Get(GetArgs),

	/// List autoenv paths.
	List(ListArgs),

	/// Remove an autoenv path.
	Remove(RemoveArgs),
}

/// Add an autoenv path.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct AddArgs {
	pub path: Option<PathBuf>,
}

/// Get the autoenv path for a path.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct GetArgs {}

/// List autoenv paths.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct ListArgs {}

/// Remove an autoenv path.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct RemoveArgs {
	pub path: Option<PathBuf>,
}

impl Cli {
	pub async fn command_autoenv(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Add(args) => self.command_autoenv_add(args).boxed(),
			Command::Get(args) => self.command_autoenv_get(args).boxed(),
			Command::List(args) => self.command_autoenv_list(args).boxed(),
			Command::Remove(args) => self.command_autoenv_remove(args).boxed(),
		}
		.await?;
		Ok(())
	}

	async fn command_autoenv_add(&self, args: AddArgs) -> Result<()> {
		// Get the path.
		let mut path = std::env::current_dir().wrap_err("Failed to get the working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}
		let path = tokio::fs::canonicalize(&path)
			.await
			.wrap_err("Failed to canonicalize the path.")?;

		// Read the config.
		let mut config = Self::read_config().await?.unwrap_or_default();

		// Add the autoenv.
		let mut autoenvs = config.autoenvs.unwrap_or_default();
		autoenvs.push(path);
		config.autoenvs = Some(autoenvs);

		// Write the config.
		Self::write_config(&config).await?;

		Ok(())
	}

	async fn command_autoenv_get(&self, _args: GetArgs) -> Result<()> {
		// Get the working directory path.
		let working_directory_path =
			std::env::current_dir().wrap_err("Failed to get the working directory.")?;

		// Read the config.
		let config = Self::read_config().await?.unwrap_or_default();

		// Get the autoenv path for the working directory path.
		let Some(autoenv_paths) = config.autoenvs.as_ref() else {
			return Ok(());
		};
		let mut autoenv_paths = autoenv_paths
			.iter()
			.filter(|path| working_directory_path.starts_with(path))
			.collect_vec();
		autoenv_paths.sort_by_key(|path| path.components().count());
		autoenv_paths.reverse();
		let Some(autoenv_path) = autoenv_paths.first() else {
			return Ok(());
		};
		let autoenv_path = *autoenv_path;

		// Print the autoenv path.
		println!("{}", autoenv_path.display());

		Ok(())
	}

	async fn command_autoenv_list(&self, _args: ListArgs) -> Result<()> {
		// Read the config.
		let config = Self::read_config().await?.unwrap_or_default();

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

	async fn command_autoenv_remove(&self, args: RemoveArgs) -> Result<()> {
		// Get the path.
		let mut path = std::env::current_dir().wrap_err("Failed to get the working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}
		let path = tokio::fs::canonicalize(&path)
			.await
			.wrap_err("Failed to canonicalize the path.")?;

		// Read the config.
		let mut config = Self::read_config().await?.unwrap_or_default();

		// Remove the autoenv.
		if let Some(mut autoenvs) = config.autoenvs {
			if let Some(index) = autoenvs.iter().position(|p| *p == path) {
				autoenvs.remove(index);
			}
			config.autoenvs = Some(autoenvs);
		}

		// Write the config.
		Self::write_config(&config).await?;

		Ok(())
	}
}
