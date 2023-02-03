use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::{compiler::ModuleIdentifier, package::PackageHash, Cli};

/// A `PackageSpecifier` represents a reference to a package, which can either be a local path or a registry package. A local path always starts with `./`, `../`, or an absolute path. Otherwise, the package specifier will be treated as a registry package, which can either be just a name (like `std`) or a name followed by a version (like `std@1.0.0`). The name by default refers to a package by name in the registry, but can also refer to a dependency key configured under `metadata.dependencies`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum PackageSpecifier {
	Path {
		path: PathBuf,
	},
	Registry {
		name: String,
		version: Option<String>,
	},
}

impl PackageSpecifier {
	#[must_use]
	pub fn key(&self) -> &str {
		match self {
			PackageSpecifier::Path { path } => path.to_str().expect("Invalid path."),
			PackageSpecifier::Registry { name, .. } => name,
		}
	}
}

impl std::str::FromStr for PackageSpecifier {
	type Err = anyhow::Error;
	fn from_str(source: &str) -> Result<PackageSpecifier> {
		if source.starts_with('.') || source.starts_with('/') {
			// Parse as a path specifier.
			let path = PathBuf::from_str(source)?;
			Ok(PackageSpecifier::Path { path })
		} else {
			// Parse as a registry specifier.
			let mut components = source.split('@');
			let name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);
			Ok(PackageSpecifier::Registry { name, version })
		}
	}
}

impl std::fmt::Display for PackageSpecifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			PackageSpecifier::Path { path } => {
				write!(f, "{}", path.display())
			},
			PackageSpecifier::Registry { name, version } => {
				write!(f, "{name}")?;
				if let Some(version) = version {
					write!(f, "@{version}")?;
				}
				Ok(())
			},
		}
	}
}

impl Cli {
	pub async fn entrypoint_module_identifier_for_specifier(
		&self,
		specifier: &PackageSpecifier,
	) -> Result<ModuleIdentifier> {
		match &specifier {
			PackageSpecifier::Path { path } => {
				let path = std::env::current_dir()
					.context("Failed to get the current directory")?
					.join(path);
				let path = tokio::fs::canonicalize(&path).await?;
				let module_identifier = ModuleIdentifier::new_path(path, "package.tg".into());
				Ok(module_identifier)
			},

			PackageSpecifier::Registry { name, version } => {
				let package_hash = self
					.get_package_hash_from_specifier(name, version.as_deref())
					.await?;
				let module_identifier =
					ModuleIdentifier::new_hash(package_hash, "package.tg".into());
				Ok(module_identifier)
			},
		}
	}

	pub async fn package_hash_for_specifier(
		&self,
		specifier: &PackageSpecifier,
		locked: bool,
	) -> Result<PackageHash> {
		match specifier {
			PackageSpecifier::Path { path } => {
				let package_hash = self.checkin_package(path, locked).await.with_context(|| {
					format!("Failed to create the package for specifier '{specifier}'.")
				})?;
				Ok(package_hash)
			},

			PackageSpecifier::Registry { name, version } => {
				let package_hash = self
					.get_package_hash_from_specifier(name, version.as_deref())
					.await?;
				Ok(package_hash)
			},
		}
	}

	#[allow(clippy::unused_async)]
	pub async fn get_package_hash_from_specifier(
		&self,
		_name: &str,
		_version: Option<&str>,
	) -> Result<PackageHash> {
		todo!()
		// let name = &package_specifier.name;
		// let version = package_specifier
		// 	.version
		// 	.as_ref()
		// 	.context("A version is required.")?;
		// let hash = self
		// 	.api_client
		// 	.get_package_version(name, version)
		// 	.await
		// 	.with_context(|| {
		// 		format!(r#"Failed to get the package "{name}" at version "{version}"."#)
		// 	})?;
		// Ok(hash)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_specifier() {
		let left: PackageSpecifier = "hello".parse().unwrap();
		let right = PackageSpecifier::Registry {
			name: "hello".to_owned(),
			version: None,
		};
		assert_eq!(left, right);

		let left: PackageSpecifier = "hello@0.0.0".parse().unwrap();
		let right = PackageSpecifier::Registry {
			name: "hello".to_owned(),
			version: Some("0.0.0".to_owned()),
		};
		assert_eq!(left, right);

		let path_specifiers = ["./hello", "./", "."];
		for path_specifier in path_specifiers {
			let left: PackageSpecifier = path_specifier.parse().unwrap();
			let right = PackageSpecifier::Path {
				path: PathBuf::from(path_specifier),
			};
			assert_eq!(left, right);
		}
	}
}
