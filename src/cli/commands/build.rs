use super::PackageArgs;
use crate::{
	error::{Error, Result, WrapErr},
	Cli,
};
use tangram::{
	function::Function,
	operation::{Call, Operation},
	package,
	util::fs,
};

/// Call a function.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(default_value = "default")]
	pub export: String,

	#[arg(short, long)]
	pub output: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self.tg.resolve_package(&args.package, None).await?;

		// Create the package instance.
		let package_instance_hash = self
			.tg
			.clone()
			.create_package_instance(&package_identifier, args.package_args.locked)
			.await
			.wrap_err("Failed to create the package instance.")?;

		// Run the operation.
		let function = Function {
			package_instance_hash,
			name: args.export,
		};
		let context = Self::create_default_context()?;
		let args_ = Vec::new();
		let operation = Operation::Call(Call {
			function,
			context,
			args: args_,
		});
		let output = operation.run(&self.tg).await?;

		// Check out the output if requested.
		if let Some(output_path) = args.output {
			let artifact_hash = output
				.as_artifact()
				.copied()
				.wrap_err("Expected the output to be an artifact.")?;
			self.tg
				.check_out_external(artifact_hash, &output_path)
				.await?;
		}

		// Print the output.
		let output = serde_json::to_string_pretty(&output).map_err(Error::other)?;
		println!("{output}");

		Ok(())
	}
}
