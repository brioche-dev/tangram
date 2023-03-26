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
#[derive(clap::Args)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	package_specifier: package::Specifier,

	#[arg(default_value = "default")]
	name: String,

	#[arg(long)]
	checkout: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Resolve the package specifier.
		let package_identifier = self
			.tg
			.resolve_package(&args.package_specifier, None)
			.await?;

		// Create the package instance.
		let package_instance_hash = self
			.tg
			.clone()
			.create_package_instance(&package_identifier, args.locked)
			.await
			.wrap_err("Failed to create the package instance.")?;

		// Run the operation.
		let function = Function {
			package_instance_hash,
			name: args.name,
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
		if let Some(path) = args.checkout {
			let artifact_hash = output
				.as_artifact()
				.wrap_err("Expected the output to be an artifact.")?;
			self.tg.check_out_external(*artifact_hash, &path).await?;
		}

		// Print the output.
		let output = serde_json::to_string_pretty(&output).map_err(Error::other)?;
		println!("{output}");

		Ok(())
	}
}
