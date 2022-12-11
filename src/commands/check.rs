use crate::{
	compiler::{self, Compiler},
	specifier::Specifier,
	Cli,
};
use anyhow::{bail, Result};
use clap::Parser;

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
		// Lock the cli.
		let cli = self.lock_shared().await?;

		// If the specifier is a path specifier, first generate its lockfile.
		if let Specifier::Path(path) = &args.specifier {
			cli.generate_lockfile(path, args.locked).await?;
		}

		// Create a compiler.
		let compiler = Compiler::new(self.clone());

		// Get the js URLs for the package.
		let urls = cli.js_urls_for_specifier(&args.specifier).await?;

		// Check the package for diagnostics.
		let diagnostics = compiler.check(urls).await?;

		// Print the diagnostics.
		let mut exit_with_error = false;
		for diagnostics in diagnostics.values() {
			for diagnostic in diagnostics {
				// If we see a diagnostic, make sure we exit with a nonzero exit code.
				exit_with_error = true;

				let compiler::types::Diagnostic {
					location, message, ..
				} = diagnostic;
				if let Some(location) = location {
					let compiler::types::Location { url, range, .. } = location;
					let compiler::types::Position { line, character } = range.start;
					let line = line + 1;
					let character = character + 1;

					match url {
						compiler::Url::PathModule(compiler::url::PathModule {
							package_path,
							module_path,
						}) => {
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

						compiler::Url::PathImport(compiler::url::PathImport {
							package_path,
							..
						})
						| compiler::Url::PathTarget(compiler::url::PathTarget {
							package_path,
							..
						}) => {
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

						_ => {
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
