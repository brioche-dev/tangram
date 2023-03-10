use crate::Cli;
use futures::FutureExt;
use indoc::indoc;
use itertools::Itertools;
use tangram::{
	error::{Context, Result},
	function::Function,
	operation::{Call, Operation},
	os, package,
};

/// Manage autoshell paths.
#[derive(clap::Args)]
pub struct Args {
	#[command(subcommand)]
	command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
	Add(AddArgs),

	List(ListArgs),

	Remove(RemoveArgs),

	#[command(hide = true)]
	Hook(HookArgs),
}

/// Add an autoshell path.
#[derive(clap::Args)]
pub struct AddArgs {
	path: Option<os::PathBuf>,
}

/// List all autoshell paths.
#[derive(clap::Args)]
pub struct ListArgs {}

/// Remove an autoshell path.
#[derive(clap::Args)]
pub struct RemoveArgs {
	path: Option<os::PathBuf>,
}

#[derive(clap::Args)]
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
			let path = path.display();
			println!("{path}");
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
		let working_directory_path =
			std::env::current_dir().context("Failed to get the working directory.")?;

		// Get the autoshell path for the working directory path.
		let Some(autoshells_paths) = config.autoshells.as_ref() else {
			return Ok(());
		};
		let mut autoshells_paths = autoshells_paths
			.iter()
			.filter(|path| working_directory_path.starts_with(path))
			.collect_vec();
		autoshells_paths.sort_by_key(|path| path.components().count());
		autoshells_paths.reverse();
		let Some(autoshell_path) = autoshells_paths.first() else {
			return Ok(());
		};
		let autoshell_path = *autoshell_path;

		// Get the package instance hash for this package.
		let package_identifier = package::Identifier::Path(autoshell_path.clone());
		let package_instance_hash = self
			.tg
			.create_package_instance(&package_identifier, false)
			.await?;

		// Create the operation.
		let function = Function {
			package_instance_hash,
			name: "shell".into(),
		};
		let context = Self::create_default_context()?;
		let args = Vec::new();
		let operation = Operation::Call(Call {
			function,
			context,
			args,
		});
		let operation_hash = self.tg.add_operation(&operation)?;

		// Run the operation.
		let output = self
			.tg
			.run(operation_hash)
			.await
			.context("Failed to run the operation.")?;

		// Get the output artifact.
		let output_artifact_hash = output
			.into_artifact()
			.context("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = self.tg.check_out_internal(output_artifact_hash).await?;

		// Get the path to the executable.
		let shell_activate_script_path = artifact_path.join("activate");

		// Print the source command.
		println!("source {}", shell_activate_script_path.to_str().unwrap());

		Ok(())
	}
}
