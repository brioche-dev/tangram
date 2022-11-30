use crate::Cli;
use anyhow::{bail, Result};
use clap::Parser;
use tangram_core::{js, specifier::Specifier};

#[derive(Parser)]
pub struct Args {
	#[arg(long)]
	locked: bool,

	#[arg(default_value = ".")]
	specifier: Specifier,

	/// Only show errors from path modules matching the given regular expression.
	#[arg(short, long)]
	filter_filenames: Option<regex::Regex>,
}

impl Cli {
	pub(crate) async fn command_check(&self, args: Args) -> Result<()> {
		// If the specifier is a path specifier, first generate its lockfile.
		if let Specifier::Path(path) = &args.specifier {
			self.builder
				.lock_shared()
				.await?
				.generate_lockfile(&self.api_client, path, args.locked)
				.await?;
		}

		// Create a compiler.
		let compiler = js::Compiler::new(self.builder.clone());

		// Get a path URL to the package.
		let url = self.js_url_for_specifier(&args.specifier).await?;

		// Check the package for diagnostics.
		let diagnostics = compiler.check(vec![url]).await?;

		// Print the diagnostics.
		let mut exit_with_error = false;
		for diagnostics in diagnostics.values() {
			for diagnostic in diagnostics {
				// If we see a diagnostic, make sure we exit with a nonzero exit code.
				exit_with_error = true;

				let js::compiler::types::Diagnostic {
					location, message, ..
				} = diagnostic;
				if let Some(location) = location {
					let js::compiler::types::Location { url, range, .. } = location;
					let js::compiler::types::Position { line, character } = range.start;
					let line = line + 1;
					let character = character + 1;

					match url {
						js::Url::PathModule {
							package_path,
							module_path,
						} => {
							// Skip diagnostics from paths that do not match the filter.
							let path = package_path.join(module_path);
							let path = path.display().to_string();
							if let Some(filter) = &args.filter_filenames {
								if !filter.is_match(&path) {
									continue;
								}
							}

							println!("{path}:{line}:{character}");
							println!("{message}");
							println!();
						},

						js::Url::PathTargets { package_path } => {
							// Skip diagnostics from paths that do not match the filter.
							let path = package_path.display().to_string();
							if let Some(filter) = &args.filter_filenames {
								if !filter.is_match(&path) {
									continue;
								}
							}

							println!("{url}:{line}:{character}");
							println!("{message}");
							println!();
						},

						js::Url::Lib { .. }
						| js::Url::PackageModule { .. }
						| js::Url::PackageTargets { .. } => {
							println!("{url}:{line}:{character}");
							println!("{message}");
							println!();
						},
					};
				}
			}
		}

		if exit_with_error {
			bail!("Some checks failed.");
		}

		Ok(())
	}
}
