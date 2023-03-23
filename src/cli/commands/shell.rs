use crate::{error::Result, Cli};
use tangram::{package, path::Path};

#[derive(clap::Args)]
#[command(
	about = r#"Build a package's "shell" export and run it."#,
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
	pub async fn command_shell(&self, mut args: Args) -> Result<()> {
		// Set the default export name to "shell".
		args.export = args.export.or_else(|| Some("shell".to_owned()));

		// Create the run args.
		let args = super::run::Args {
			executable_path: args.executable_path,
			locked: args.locked,
			export: args.export,
			package_specifier: args.package_specifier,
			trailing_args: args.trailing_args,
		};

		// Run!
		self.command_run(args).await?;

		Ok(())
	}
}
