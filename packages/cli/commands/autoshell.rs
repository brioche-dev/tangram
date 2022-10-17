use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::path::PathBuf;

#[derive(Parser)]
#[command(long_about = "Manage autoshells.")]
pub struct Args {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	Add(AddArgs),
	List(ListArgs),
	Remove(RemoveArgs),
}

#[derive(Parser, Debug)]
#[command(long_about = "Add a path as an autoshell.")]
pub struct AddArgs {
	path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
#[command(long_about = "List all autoshells.")]
pub struct ListArgs {}

#[derive(Parser, Debug)]
#[command(long_about = "Remove a path as an autoshells.")]
pub struct RemoveArgs {
	path: Option<PathBuf>,
}

impl Cli {
	pub(crate) async fn command_autoshell(&self, args: Args) -> Result<()> {
		match args.subcommand {
			Subcommand::Add(args) => self.command_autoshell_add(args).boxed(),
			Subcommand::List(args) => self.command_autoshell_list(args).boxed(),
			Subcommand::Remove(args) => self.command_autoshell_remove(args).boxed(),
		}
		.await?;
		Ok(())
	}

	async fn command_autoshell_add(&self, args: AddArgs) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}
		let path = tokio::fs::canonicalize(&path)
			.await
			.context("Failed to canonicalize the path.")?;

		// Read the config.
		let mut config = Cli::read_config().await?.unwrap_or_default();

		// Add the autoshell.
		let mut autoshells = config.autoshells.unwrap_or_default();
		autoshells.push(path);
		config.autoshells = Some(autoshells);

		// Write the config.
		Cli::write_config(&config).await?;

		Ok(())
	}

	async fn command_autoshell_list(&self, _args: ListArgs) -> Result<()> {
		// Read the config.
		let config = Cli::read_config().await?.unwrap_or_default();

		// List the autoshells.
		for path in config.autoshells.iter().flatten() {
			println!("{}", path.display());
		}

		Ok(())
	}

	async fn command_autoshell_remove(&self, args: RemoveArgs) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().context("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}
		let path = tokio::fs::canonicalize(&path)
			.await
			.context("Failed to canonicalize the path.")?;

		// Read the config.
		let mut config = Cli::read_config().await?.unwrap_or_default();

		// Remove the autoshell.
		if let Some(mut autoshells) = config.autoshells {
			if let Some(index) = autoshells.iter().position(|p| *p == path) {
				autoshells.remove(index);
			}
			config.autoshells = Some(autoshells);
		}

		// Write the config.
		Cli::write_config(&config).await?;

		Ok(())
	}
}
