use super::PackageArgs;
use crate::{
	error::{Error, Result, WrapErr},
	Cli,
};
use tangram::{
	call::Call,
	function::Function,
	package::{self, Package},
	util::fs,
};

/// Call a function.
#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct Args {
	#[arg(short, long, default_value = ".")]
	pub package: package::Specifier,

	#[command(flatten)]
	pub package_args: PackageArgs,

	#[arg(default_value = "default")]
	pub function: String,

	#[arg(short, long)]
	pub output: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_build(&self, args: Args) -> Result<()> {
		// Get the package.
		let package = Package::with_specifier(&self.tg, args.package).await?;

		// Create the package instance.
		let package_instance = package
			.instantiate(&self.tg)
			.await
			.wrap_err("Failed to create the package instance.")?;

		// Run the operation.
		let function = Function::new(&package_instance, args.function);
		let env = Self::create_default_env()?;
		let args_ = Vec::new();
		let call = Call::new(&self.tg, function, env, args_)
			.await
			.wrap_err("Failed to create the call.")?;
		let output = call.run(&self.tg).await?;

		// Check out the output if requested.
		if let Some(path) = args.output {
			let artifact = output
				.as_artifact()
				.wrap_err("Expected the output to be an artifact.")?;
			artifact
				.check_out(&self.tg, &path)
				.await
				.wrap_err("Failed to check out the artifact.")?;
		}

		// Print the output.
		let output = serde_json::to_string_pretty(&output).map_err(Error::other)?;
		println!("{output}");

		Ok(())
	}
}
