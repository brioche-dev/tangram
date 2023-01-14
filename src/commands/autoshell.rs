use crate::{
	operation::{Operation, Target},
	Cli,
};
use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use indoc::indoc;
use itertools::Itertools;
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Manage autoshell paths.")]
pub struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(Parser)]
pub enum Command {
	Add(AddArgs),
	List(ListArgs),
	Remove(RemoveArgs),
	#[command(hide = true)]
	Hook(HookArgs),
}

#[derive(Parser, Debug)]
#[command(about = "Add an autoshell path.")]
pub struct AddArgs {
	path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
#[command(about = "List all autoshell paths.")]
pub struct ListArgs {}

#[derive(Parser, Debug)]
#[command(about = "Remove an autoshell path.")]
pub struct RemoveArgs {
	path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
#[command(about = "Hook")]
pub struct HookArgs {
	shell: String,
}

impl Cli {
	pub async fn command_autoshell(&self, args: Args) -> Result<()> {
		match args.command {
			Command::Add(args) => self.command_autoshell_add(args).boxed(),
			Command::List(args) => self.command_autoshell_list(args).boxed(),
			Command::Remove(args) => self.command_autoshell_remove(args).boxed(),
			Command::Hook(args) => self.command_autoshell_hook(args).boxed(),
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
		let mut config = self.read_config().await?.unwrap_or_default();

		// Add the autoshell.
		let mut autoshells = config.autoshells.unwrap_or_default();
		autoshells.push(path);
		config.autoshells = Some(autoshells);

		// Write the config.
		self.write_config(&config).await?;

		Ok(())
	}

	async fn command_autoshell_list(&self, _args: ListArgs) -> Result<()> {
		// Read the config.
		let config = self.read_config().await?.unwrap_or_default();

		// List the autoshells.
		let autoshells = config.autoshells.unwrap_or_default();

		if autoshells.is_empty() {
			eprintln!("There are no autoshells.");
		}

		for path in autoshells {
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
		let mut config = self.read_config().await?.unwrap_or_default();

		// Remove the autoshell.
		if let Some(mut autoshells) = config.autoshells {
			if let Some(index) = autoshells.iter().position(|p| *p == path) {
				autoshells.remove(index);
			}
			config.autoshells = Some(autoshells);
		}

		// Write the config.
		self.write_config(&config).await?;

		Ok(())
	}

	async fn command_autoshell_hook(&self, _args: HookArgs) -> Result<()> {
		// Read the config.
		let config = self.read_config().await?.unwrap_or_default();

		// Deactivate an existing autoshell.
		let program = indoc!(
			r#"
				type _tangram_deactivate &> /dev/null && _tangram_deactivate
			"#
		);
		print!("{program}");

		// Get the current working directory.
		let cwd = std::env::current_dir().context("Failed to get the working directory.")?;

		// Get the autoshells.
		let Some(autoshells) = config.autoshells.as_ref() else {
			return Ok(());
		};

		// Get the autoshells for the path.
		let mut autoshells_paths = autoshells
			.iter()
			.filter(|path| cwd.starts_with(path))
			.collect_vec();
		autoshells_paths.sort_by_key(|path| path.components().count());

		let Some(autoshell) = autoshells_paths.last() else {
			return Ok(());
		};

		// Check in the package for this autoshell.
		let package_hash = self.checkin_package(autoshell, false).await?;

		// Create the target args.
		let target_args = self.create_target_args(None)?;

		// Create the operation.
		let operation = Operation::Target(Target {
			package: package_hash,
			name: "shell".into(),
			args: target_args,
		});

		// Run the operation.
		let output = self
			.run(&operation)
			.await
			.context("Failed to run the operation.")?;

		// Get the output artifact.
		let output_artifact_hash = output
			.into_artifact()
			.context("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = self.checkout_internal(output_artifact_hash).await?;

		// Get the path to the executable.
		let shell_activate_script_path = artifact_path.join("activate");

		// Print the source command.
		println!("source {}", shell_activate_script_path.to_str().unwrap());

		Ok(())
	}
}
