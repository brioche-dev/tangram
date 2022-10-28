use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use indoc::indoc;
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
	#[command(hide = true)]
	Hook(HookArgs),
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

#[derive(Parser, Debug)]
#[command(long_about = "Hook")]
pub struct HookArgs {
	shell: String,
}

impl Cli {
	pub(crate) async fn command_autoshell(&self, args: Args) -> Result<()> {
		match args.subcommand {
			Subcommand::Add(args) => self.command_autoshell_add(args).boxed(),
			Subcommand::List(args) => self.command_autoshell_list(args).boxed(),
			Subcommand::Remove(args) => self.command_autoshell_remove(args).boxed(),
			Subcommand::Hook(args) => self.command_autoshell_hook(args).boxed(),
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

	async fn command_autoshell_hook(&self, _args: HookArgs) -> Result<()> {
		// Read the config.
		let config = Cli::read_config().await?.unwrap_or_default();

		// Deactivate an existing autoshell.
		let program = indoc!(
			r#"
				type _tangram_deactivate &> /dev/null && _tangram_deactivate
			"#
		);
		print!("{}", program);

		// Get the current working directory.
		let cwd = std::env::current_dir().context("Failed to get the working directory.")?;

		// Get the autoshells.
		let autoshells = if let Some(autoshells) = config.autoshells.as_ref() {
			autoshells
		} else {
			return Ok(());
		};

		// Get the autoshells for .
		let mut autoshells: Vec<_> = autoshells
			.iter()
			.filter(|path| cwd.starts_with(path))
			.collect();
		autoshells.sort_by_key(|path| path.components().count());

		let autoshell = if let Some(autoshell) = autoshells.last() {
			autoshell
		} else {
			return Ok(());
		};

		// Lock the builder.
		let builder = self.builder.lock_shared().await?;

		// Check in the package for this autoshell.
		let package_hash = builder
			.checkin_package(&self.api_client, autoshell, false)
			.await?;

		// Create the target args.
		let target_args = self.create_target_args(None).await?;

		// Add the expression.
		let expression_hash = builder
			.add_expression(&tangram_core::expression::Expression::Target(
				tangram_core::expression::Target {
					package: package_hash,
					name: "shell".into(),
					args: target_args,
				},
			))
			.await?;

		// Evaluate the expression.
		let output_hash = builder
			.evaluate(expression_hash, expression_hash)
			.await
			.context("Failed to evaluate the target expression.")?;

		// Check out the artifact.
		let artifact_path = builder.checkout_to_artifacts(output_hash).await?;

		// Get the path to the executable.
		let shell_activate_script_path = artifact_path.join("activate");

		// Print the source command.
		println!("source {}", shell_activate_script_path.to_str().unwrap());

		Ok(())
	}
}
