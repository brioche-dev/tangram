use crate::{
	error::{Result, WrapErr},
	Cli,
};
use std::os::unix::process::CommandExt;
use tangram::{
	function::Function,
	operation::{Call, Operation},
	package,
	path::Path,
};

#[derive(Debug, clap::Args)]
#[command(
	about = "Build a package and run an executable from its output.",
	trailing_var_arg = true
)]
pub struct Args {
	#[arg(long)]
	pub executable_path: Option<Path>,
	#[arg(long)]
	pub locked: bool,
	#[arg(long)]
	pub export: Option<String>,

	#[arg(default_value = ".")]
	pub package_specifier: package::Specifier,

	pub trailing_args: Vec<String>,
}

impl Cli {
	pub async fn command_run(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self
			.tg
			.resolve_package(&args.package_specifier, None)
			.await?;

		// Get the package instance hash.
		let package_instance_hash = self
			.tg
			.clone()
			.create_package_instance(&package_identifier, args.locked)
			.await?;

		// Get the export name.
		let name = args.export.unwrap_or_else(|| "default".to_owned());

		// Run the operation.
		let function = Function {
			package_instance_hash,
			name,
		};
		let context = Self::create_default_context()?;
		let args_ = Vec::new();
		let operation = Operation::Call(Call {
			function,
			context,
			args: args_,
		});
		let output = operation.run(&self.tg).await?;

		// Get the output artifact.
		let output_artifact_hash = output
			.into_artifact()
			.wrap_err("Expected the output to be an artifact.")?;

		// Check out the artifact.
		let artifact_path = self.tg.check_out_internal(output_artifact_hash).await?;

		// Get the executable path.
		let executable_path = args
			.executable_path
			.unwrap_or_else(|| "run".parse().unwrap());

		// Get the path to the executable.
		let executable_path = artifact_path.join(executable_path.to_string());

		// Exec the process.
		Err(std::process::Command::new(executable_path)
			.args(args.trailing_args)
			.exec()
			.into())
	}
}
