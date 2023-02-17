use crate::{
	artifact::{self, Artifact},
	directory::Directory,
	module, os, package, Cli,
};
use anyhow::Result;
use std::sync::Arc;

pub struct Output {
	pub package_hash: artifact::Hash,
	pub dependency_specifiers: Vec<package::Specifier>,
}

impl Cli {
	/// Check in the package at the specified path.
	#[allow(clippy::unused_async)]
	pub async fn check_in_package(self: &Arc<Self>, path: &os::Path) -> Result<Output> {
		// Create a queue of modules to visit.
		let root_module_identifier = module::Identifier::for_root_module_in_package_at_path(path);
		let mut module_identifier_queue = vec![root_module_identifier];

		// Create the package.
		let mut directory = Directory::new();

		// Track the dependency package specifiers.
		let mut dependency_specifiers = Vec::new();

		// Visit each module.
		while let Some(module_identifier) = module_identifier_queue.pop() {
			let (module::Identifier::Normal(module::identifier::Normal {
				source: module::identifier::Source::Path(package_path),
				path: module_path,
			})
			| module::Identifier::Artifact(module::identifier::Artifact {
				source: module::identifier::Source::Path(package_path),
				path: module_path,
			})) = &module_identifier else {
				continue;
			};

			// Check in the artifact at the imported path.
			let imported_artifact_hash = self
				.check_in(&package_path.join(module_path.to_string()))
				.await?;

			// Add the imported artifact to the directory.
			self.directory_add(&mut directory, module_path, imported_artifact_hash)
				.await?;

			// If the module is a normal module, then explore its imports.
			if let module::Identifier::Normal(module::identifier::Normal { path, .. }) =
				&module_identifier
			{
				// Load the module.
				let module_text = self.load_module(&module_identifier).await?;

				// Get the module's imports.
				let module_specifiers = self.imports(&module_text).await?;

				// Handle each module specifier.
				for module_specifier in module_specifiers {
					// Resolve.

					match module_specifier {
						// If the module is specified with a path, then add the resolved module to the queue.
						module::Specifier::Path(specifier_path) => {
							let resolved_module_identifier =
								Self::resolve_module_with_path_specifier(
									&specifier_path,
									&module_identifier,
								)
								.await?;
							module_identifier_queue.push(resolved_module_identifier);
						},

						// If the module is specified with a package, then add the specifier to the list of dependencies.
						module::Specifier::Package(package_specifier) => {
							// If the specifier is a path specifier, then resolve it relative to the package root.
							let package_specifier = match package_specifier {
								package::Specifier::Path(specifier_path) => {
									let specifier_path =
										specifier_path.display().to_string().into();
									let resolved_path = path.join(&specifier_path).normalize();
									package::Specifier::Path(resolved_path.to_string().into())
								},

								package::Specifier::Registry(_) => package_specifier,
							};

							dependency_specifiers.push(package_specifier);
						},
					};
				}
			}
		}

		// Add the artifact.
		let package_hash = self.add_artifact(&Artifact::Directory(directory)).await?;

		// Create the output.
		let output = Output {
			package_hash,
			dependency_specifiers,
		};

		Ok(output)
	}
}
