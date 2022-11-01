use crate::Cli;
use anyhow::Result;
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
		let compiler = js::Compiler::new(self.builder.clone());

		// Create the URL.
		let url = self.js_url_for_specifier(&args.specifier).await?;

		// Check the package for diagnostics.
		let diagnostics = compiler.check(vec![url]).await?;

		// Print the diagnostics.
		for diagnostics in diagnostics.values() {
			for diagnostic in diagnostics {
				let js::compiler::types::Diagnostic {
					location, message, ..
				} = diagnostic;
				if let Some(location) = location {
					let js::compiler::types::Location { url, range, .. } = location;
					let js::compiler::types::Position { line, character } = range.start;
					let line = line + 1;
					let character = character + 1;
					if let js::Url::PathModule {
						package_path,
						module_path,
					} = url
					{
						let path = package_path.join(module_path);
						let path = path.display();
						println!("{path}:{line}:{character}");
						println!("{message}");
						println!();
					}
				}
			}
		}

		Ok(())
	}
}
