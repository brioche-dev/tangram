use crate::{
	api_client::ApiClient,
	builder::Shared,
	expression::{Expression, Package},
	hash::Hash,
	lockfile::{self, Lockfile},
	manifest::Manifest,
};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use std::{collections::BTreeMap, path::Path};
use tokio::io::AsyncReadExt;

impl Shared {
	/// Check in a package from the provided source path.
	pub async fn checkin_package(
		&self,
		api_client: &ApiClient,
		path: &Path,
		locked: bool,
	) -> Result<Hash> {
		// Generate the lockfile if necessary.
		if !locked {
			self.generate_lockfile(api_client, path, locked)
				.await
				.context("Failed to generate the lockfile.")?;
		}

		// Check in the package source.
		let package_source_hash = self
			.checkin(path)
			.await
			.context("Failed to check in the package.")?;

		// Read the lockfile.
		let lockfile_path = path.join("tangram.lock");
		let lockfile = tokio::fs::read(&lockfile_path)
			.await
			.context("Failed to read the lockfile.")?;
		let lockfile: Lockfile =
			serde_json::from_slice(&lockfile).context("Failed to deserialize the lockfile.")?;

		// Create the package expression.
		let dependencies = lockfile
			.as_v1()
			.context("Expected V1 Lockfile.")?
			.dependencies
			.iter()
			.map(|(name, entry)| (name.clone().into(), entry.hash))
			.collect();
		let package = Package {
			source: package_source_hash,
			dependencies,
		};
		let hash = self.add_expression(&Expression::Package(package)).await?;

		Ok(hash)
	}

	#[async_recursion]
	#[must_use]
	pub async fn generate_lockfile(
		&self,
		api_client: &ApiClient,
		path: &Path,
		locked: bool,
	) -> Result<()> {
		// Read the manifest.
		let manifest_path = path.join("tangram.json");
		let manifest = tokio::fs::read(&manifest_path)
			.await
			.context("Failed to read the package manifest.")?;
		let manifest: Manifest = serde_json::from_slice(&manifest).with_context(|| {
			format!(
				r#"Failed to parse the package manifest at path "{}"."#,
				manifest_path.display()
			)
		})?;

		// Get the dependencies.
		let mut dependencies = BTreeMap::new();
		for (dependency_name, dependency) in manifest.dependencies.iter().flatten() {
			// Retrieve the path dependency.
			let entry = match dependency {
				crate::manifest::Dependency::PathDependency(dependency) => {
					// Get the absolute path to the dependency.
					let dependency_path = path.join(&dependency.path);
					let dependency_path = tokio::fs::canonicalize(&dependency_path).await?;

					// Get the dependency's hash.
					let dependency_hash = self
						.checkin_package(api_client, &dependency_path, locked)
						.await
						.context("Failed to check in the dependency.")?;

					// Get the dependency package.
					let dependency_package = self
						.get_expression_local(dependency_hash)?
						.into_package()
						.context("The dependency must be a package.")?;

					// Create the lockfile entry.
					lockfile::Dependency {
						hash: dependency_hash,
						source: dependency_package.source,
						dependencies: None,
					}
				},

				// Handle a registry dependency.
				crate::manifest::Dependency::RegistryDependency(dependency) => {
					// Get the package hash from the registry.
					let dependency_version = &dependency.version;
					let dependency_hash = api_client
								.get_package_version(dependency_name, &dependency.version)
								.await
								.with_context(||
									format!(r#"Package with name "{dependency_name}" and version "{dependency_version}" is not in the package registry."#)
								)?;
					let dependency_source_hash = self.get_package_source(dependency_hash)?;

					// Create the lockfile Entry.
					lockfile::Dependency {
						hash: dependency_hash,
						source: dependency_source_hash,
						dependencies: None,
					}
				},
			};

			// Add the dependency.
			dependencies.insert(dependency_name.clone(), entry);
		}

		// Create and write the lockfile.
		let lockfile = Lockfile::new_v1(dependencies);
		let lockfile =
			serde_json::to_vec_pretty(&lockfile).context("Failed to serialize the lockfile.")?;
		let lockfile_path = path.join("tangram.lock");
		tokio::fs::write(&lockfile_path, lockfile)
			.await
			.context("Failed to write the lockfile.")?;

		Ok(())
	}

	pub fn get_package_source(&self, package_hash: Hash) -> Result<Hash> {
		let package = self
			.get_expression_local(package_hash)?
			.into_package()
			.context("Expected package.")?;
		let package_source = package.source;
		Ok(package_source)
	}

	pub async fn get_package_manifest(&self, package_hash: Hash) -> Result<Manifest> {
		let package_source_hash = self.get_package_source(package_hash)?;

		let source_artifact = self
			.get_expression_local(package_source_hash)?
			.into_artifact()
			.context("Expected an artifact.")?;

		let source_directory = self
			.get_expression_local(source_artifact.root)?
			.into_directory()
			.context("Expected a directory.")?;

		let manifest_hash = source_directory
			.entries
			.get("tangram.json")
			.copied()
			.context("The package source does not contain a manifest.")?;

		let manifest_blob_hash = self
			.get_expression_local(manifest_hash)?
			.as_file()
			.context("Expected the manifest to be a file.")?
			.blob;

		let mut manifest = self
			.get_blob(manifest_blob_hash)
			.await
			.context("Failed to get the manifest blob.")?;
		let mut manifest_bytes = Vec::new();
		manifest
			.read_to_end(&mut manifest_bytes)
			.await
			.context("Failed to read the manifest.")?;
		let manifest: Manifest = serde_json::from_slice(&manifest_bytes)
			.context(r#"Failed to parse the package manifest."#)?;

		Ok(manifest)
	}
}
