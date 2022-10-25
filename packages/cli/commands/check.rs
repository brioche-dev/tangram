use crate::Cli;
use anyhow::{Context, Result};
use clap::Parser;
use tangram_core::{js, specifier::Specifier};

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: Specifier,
}

impl Cli {
	pub(crate) async fn command_check(&self, args: Args) -> Result<()> {
		// Create a compiler.
		let main_runtime_handle = tokio::runtime::Handle::current();
		let compiler = js::Compiler::new(self.builder.clone(), main_runtime_handle);

		// Create the URL.
		let url = match &args.specifier {
			Specifier::Path(path) => {
				let path = std::env::current_dir()
					.context("Failed to get the current directory")?
					.join(path);
				let path = tokio::fs::canonicalize(&path).await?;
				js::Url::new_path_targets(path)
			},
			Specifier::Registry(_) => {
				let package_hash = self
					.package_hash_for_specifier(&args.specifier, args.locked)
					.await?;
				js::Url::new_package_targets(package_hash)
			},
		};

		// Check the package for diagnostics.
		let diagnostics = compiler.check(vec![url]).await?;

		// Print the diagnostics.
		for diagnostics in diagnostics.values() {
			for diagnostic in diagnostics {
				let js::compiler::Diagnostic { location, message } = diagnostic;
				if let Some(location) = location {
					let js::compiler::DiagnosticLocation { path, range, .. } = location;
					let url = js::Url::from_typescript_path(path).await?;
					let js::compiler::Position { line, character } = range.start;
					let line = line + 1;
					let character = character + 1;
					match url {
						js::Url::PathModule { package_path, .. }
						| js::Url::PathTargets { package_path } => {
							let package_path = package_path.display();
							println!("{package_path}:{line}:{character}");
						},
						_ => {
							println!("{url}:{line}:{character}");
						},
					}
				}
				println!("{message}");
				println!();
			}
		}

		Ok(())
	}
}
