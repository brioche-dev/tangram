use super::dependency;
use crate::{
	artifact::{self, Artifact},
	directory::Directory,
	error::{return_error, Result},
	module, path,
	util::fs,
	Instance,
};
use std::{collections::HashSet, sync::Arc};

pub struct Output {
	pub package_hash: artifact::Hash,
	pub dependency_specifiers: Vec<dependency::Specifier>,
}

impl Instance {
	/// Check in the package at the specified path.
	#[allow(clippy::unused_async)]
	pub async fn check_in_package(self: &Arc<Self>, path: &fs::Path) -> Result<Output> {
		// Create a queue of modules to visit and a visited set.
		let root_module_identifier = module::Identifier::for_root_module_in_package_at_path(path);
		let mut queue = vec![root_module_identifier];
		let mut visited: HashSet<module::Identifier> = HashSet::default();

		// Create the package.
		let mut directory = Directory::new();

		// Track the dependency specifiers.
		let mut dependency_specifiers = Vec::new();

		// Visit each module.
		while let Some(module_identifier) = queue.pop() {
			// Add the module to the visited set.
			visited.insert(module_identifier.clone());

			// Get the package path from the module identifier.
			let module::identifier::Source::Path(package_path) = &module_identifier.source else {
				return_error!("Invalid module identifier.");
			};

			// Check in the artifact at the imported path.
			let imported_artifact_hash = self
				.check_in(&package_path.join(module_identifier.path.to_string()))
				.await?;

			// Add the imported artifact to the directory.
			directory
				.add(self, &module_identifier.path, imported_artifact_hash)
				.await?;

			// Load the module.
			let module_text = self.load_module(&module_identifier).await?;

			// Get the module's imports.
			let imports = self.imports(&module_text).await?;

			// Handle each import.
			for specifier in imports.imports {
				// Resolve the specifier.
				let resolved_module_identifier =
					self.resolve_module(&specifier, &module_identifier).await?;

				match specifier {
					// If the module is specified with a path, then add the resolved module identifier to the queue if it has not been visited.
					module::Specifier::Path(_) => {
						if !visited.contains(&resolved_module_identifier) {
							queue.push(resolved_module_identifier);
						}
					},

					// If the module is specified with a package, then add the specifier to the list of dependencies.
					module::Specifier::Dependency(dependency_specifier) => {
						// Convert the module dependency specifier to a package dependency specifier.
						let package_specifier = dependency_specifier
							.to_package_dependency_specifier(&module_identifier)?;

						// Add the package specifier to the list of dependencies.
						dependency_specifiers.push(package_specifier);
					},
				};
			}

			// Handle each include.
			for path in imports.includes {
				// Check in the artifact at the included path.
				let mut included_artifact_path = module_identifier.path.clone();
				included_artifact_path.push(path::Component::ParentDir);
				included_artifact_path.join(path.clone());
				let included_artifact_hash = self
					.check_in(&package_path.join(included_artifact_path.to_string()))
					.await?;

				// Add the included artifact to the directory.
				directory.add(self, &path, included_artifact_hash).await?;
			}
		}

		// Add the artifact.
		let package_hash = self.add_artifact(&Artifact::Directory(directory)).await?;

		// Add a package tracker.
		self.add_package_tracker(package_hash, path.to_owned())
			.await;

		// Create the output.
		let output = Output {
			package_hash,
			dependency_specifiers,
		};

		Ok(output)
	}
}
