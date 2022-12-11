use crate::{
	compiler,
	expression::Expression,
	hash::Hash,
	specifier::{self, Specifier},
	system::System,
	State,
};
use anyhow::{bail, Context, Result};
use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use std::{collections::BTreeMap, fs::Metadata, path::Path};

impl State {
	pub async fn create_target_args(&self, system: Option<System>) -> Result<Hash> {
		let mut arg = BTreeMap::new();
		let system = if let Some(system) = system {
			system
		} else {
			System::host()?
		};
		let system = self
			.add_expression(&Expression::String(system.to_string().into()))
			.await?;
		arg.insert("target".into(), system);
		let arg = self.add_expression(&Expression::Map(arg)).await?;
		let args = vec![arg];
		let args = self.add_expression(&Expression::Array(args)).await?;
		Ok(args)
	}
}

impl State {
	pub async fn js_urls_for_specifier(&self, specifier: &Specifier) -> Result<Vec<compiler::Url>> {
		match &specifier {
			Specifier::Package(package_specifier) => {
				let package_hash = self
					.get_package_hash_from_specifier(package_specifier)
					.await?;
				let url = compiler::Url::new_hash_module(package_hash, "tangram.ts".into());
				Ok(vec![url])
			},

			Specifier::Path(path) => {
				let path = std::env::current_dir()
					.context("Failed to get the current directory")?
					.join(path);
				let path = tokio::fs::canonicalize(&path).await?;
				let url = compiler::Url::new_path_module(path, "tangram.ts".into());
				Ok(vec![url])
			},
		}
	}

	pub async fn package_hash_for_specifier(
		&self,
		specifier: &Specifier,
		locked: bool,
	) -> Result<Hash> {
		match specifier {
			Specifier::Package(package_specifier) => {
				let package_hash = self
					.get_package_hash_from_specifier(package_specifier)
					.await?;
				Ok(package_hash)
			},

			Specifier::Path(path) => {
				let package_hash = self.checkin_package(path, locked).await.with_context(|| {
					format!("Failed to create the package for specifier '{specifier}'.")
				})?;
				Ok(package_hash)
			},
		}
	}

	pub async fn get_package_hash_from_specifier(
		&self,
		package_specifier: &specifier::Package,
	) -> Result<Hash> {
		let name = &package_specifier.name;
		let version = package_specifier
			.version
			.as_ref()
			.context("A version is required.")?;
		let hash = self
			.api_client
			.get_package_version(name, version)
			.await
			.with_context(|| {
				format!(r#"Failed to get the package "{name}" at version "{version}"."#)
			})?;
		Ok(hash)
	}
}

#[must_use]
pub fn normalize(path: &Utf8Path) -> Utf8PathBuf {
	let mut normalized_path = Utf8PathBuf::new();

	for component in path.components() {
		match component {
			Utf8Component::Prefix(prefix) => {
				// Replace the path.
				normalized_path = Utf8PathBuf::from(prefix.to_string());
			},

			Utf8Component::RootDir => {
				// Replace the path.
				normalized_path = Utf8PathBuf::from("/");
			},

			Utf8Component::CurDir => {
				// Skip current dir components.
			},

			Utf8Component::ParentDir => {
				if normalized_path.components().count() == 1
					&& matches!(
						normalized_path.components().next(),
						Some(Utf8Component::Prefix(_) | Utf8Component::RootDir)
					) {
					// If the normalized path has one component which is a prefix or a root dir component, then do nothing.
				} else if normalized_path
					.components()
					.all(|component| matches!(component, Utf8Component::ParentDir))
				{
					// If the normalized path is zero or more parent dir components, then add a parent dir component.
					normalized_path.push("..");
				} else {
					// Otherwise, remove the last component.
					normalized_path.pop();
				}
			},

			Utf8Component::Normal(string) => {
				// Add the component.
				normalized_path.push(string);
			},
		}
	}

	normalized_path
}

pub async fn path_exists(path: &Path) -> Result<bool> {
	match tokio::fs::metadata(&path).await {
		Ok(_) => Ok(true),
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
		Err(error) => Err(error.into()),
	}
}

pub async fn rmrf(path: &Path, metadata: Option<Metadata>) -> Result<()> {
	let metadata = if let Some(metadata) = metadata {
		metadata
	} else {
		tokio::fs::metadata(path).await?
	};

	if metadata.is_dir() {
		tokio::fs::remove_dir_all(path).await?;
	} else if metadata.is_file() {
		tokio::fs::remove_file(path).await?;
	} else if metadata.is_symlink() {
		tokio::fs::remove_file(path).await?;
	} else {
		bail!("The path must point to a directory, file, or symlink.");
	};

	Ok(())
}
