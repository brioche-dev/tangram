use super::Identifier;
pub use crate::package::specifier::Registry;
use crate::{
	error::{return_error, Error, Result, WrapErr},
	module,
	path::Path,
};
use std::{collections::HashSet, sync::Arc};

/// A reference from a package to a dependency, either at a path or from the registry.
#[derive(
	Clone,
	Debug,
	PartialOrd,
	Ord,
	PartialEq,
	Eq,
	Hash,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub enum Specifier {
	/// A reference to a dependency at a path.
	Path(Path),

	/// A reference to a dependency from the registry.
	Registry(Registry),
}

impl std::str::FromStr for Specifier {
	type Err = Error;

	fn from_str(value: &str) -> Result<Specifier> {
		if value.starts_with('.') {
			// If the string starts with `.`, then parse the string as a path.
			let specifier = value.parse()?;
			Ok(Specifier::Path(specifier))
		} else {
			// Otherwise, parse the string as a registry specifier.
			let specifier = value.parse()?;
			Ok(Specifier::Registry(specifier))
		}
	}
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Specifier::Path(path) => {
				write!(f, "{path}")?;
				Ok(())
			},

			Specifier::Registry(specifier) => {
				write!(f, "{specifier}")?;
				Ok(())
			},
		}
	}
}

impl TryFrom<String> for Specifier {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Specifier> for String {
	fn from(value: Specifier) -> Self {
		value.to_string()
	}
}

impl crate::Instance {
	pub async fn get_package_dependency_specifiers(
		self: &Arc<Self>,
		package_identifier: &Identifier,
	) -> Result<Vec<crate::package::dependency::Specifier>> {
		// Create a queue of modules to visit and a visited set.
		let root_module_identifier =
			module::Identifier::for_root_module_in_package(package_identifier.clone());
		let mut queue = vec![root_module_identifier];
		let mut visited: HashSet<module::Identifier> = HashSet::default();

		// Track the dependency specifiers.
		let mut dependency_specifiers = Vec::new();

		// Visit each module.
		while let Some(module_identifier) = queue.pop() {
			// Add the module to the visited set.
			visited.insert(module_identifier.clone());

			// Get the package path from the module identifier.
			let module::identifier::Source::Package(package_path) = &module_identifier.source else {
				return_error!("Invalid module identifier.");
			};

			// Load the module.
			let module_text = self
				.load_module(&module_identifier)
				.await
				.wrap_err_with(|| format!(r#"Failed to load the module "{module_identifier}"."#))?;

			// Get the module's imports.
			let imports = self
				.imports(&module_text)
				.await
				.wrap_err_with(|| "Failed to get the module's imports.")?;

			// Handle each import.
			for specifier in imports.imports {
				match specifier {
					// If the module is specified with a path, then add the resolved module identifier to the queue if it has not been visited.
					module::Specifier::Path(_) => {
						// Resolve the specifier.
						let resolved_module_identifier =
							self.resolve_module(&specifier, &module_identifier).await?;

						// Add the resolved module identifier to the queue if it has not been visited.
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
		}

		Ok(dependency_specifiers)
	}
}
