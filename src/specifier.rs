use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::{compiler::ModuleIdentifier, package::PackageHash, Cli};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Specifier {
	Path {
		path: PathBuf,
	},
	Registry {
		name: String,
		version: Option<String>,
	},
}

impl std::str::FromStr for Specifier {
	type Err = anyhow::Error;
	fn from_str(source: &str) -> Result<Specifier> {
		if source.starts_with('.') || source.starts_with('/') {
			// Parse as a path specifier.
			let path = PathBuf::from_str(source)?;
			Ok(Specifier::Path { path })
		} else {
			// Parse as a registry specifier.
			let mut components = source.split('@');
			let name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);
			Ok(Specifier::Registry { name, version })
		}
	}
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Specifier::Path { path } => {
				write!(f, "{}", path.display())
			},
			Specifier::Registry { name, version } => {
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
		specifier: &Specifier,
	) -> Result<ModuleIdentifier> {
		match &specifier {
			Specifier::Path { path } => {
				let path = std::env::current_dir()
					.context("Failed to get the current directory")?
					.join(path);
				let path = tokio::fs::canonicalize(&path).await?;
				let module_identifier = ModuleIdentifier::new_path(path, "package.tg".into());
				Ok(module_identifier)
			},

			Specifier::Registry { name, version } => {
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
		specifier: &Specifier,
		locked: bool,
	) -> Result<PackageHash> {
		match specifier {
			Specifier::Path { path } => {
				let package_hash = self.checkin_package(path, locked).await.with_context(|| {
					format!("Failed to create the package for specifier '{specifier}'.")
				})?;
				Ok(package_hash)
			},

			Specifier::Registry { name, version } => {
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
		let left: Specifier = "hello".parse().unwrap();
		let right = Specifier::Registry {
			name: "hello".to_owned(),
			version: None,
		};
		assert_eq!(left, right);

		let left: Specifier = "hello@0.0.0".parse().unwrap();
		let right = Specifier::Registry {
			name: "hello".to_owned(),
			version: Some("0.0.0".to_owned()),
		};
		assert_eq!(left, right);

		let path_specifiers = ["./hello", "./", "."];
		for path_specifier in path_specifiers {
			let left: Specifier = path_specifier.parse().unwrap();
			let right = Specifier::Path {
				path: PathBuf::from(path_specifier),
			};
			assert_eq!(left, right);
		}
	}
}
