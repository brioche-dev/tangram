use crate::{
	lock::Lock,
	lockfile::{self, Lockfile},
	manifest::Manifest,
	Cli,
};
use anyhow::{bail, Context, Result};
use async_recursion::async_recursion;
use std::{collections::BTreeMap, path::Path};
use tokio::io::AsyncReadExt;

impl Cli {
	#[async_recursion]
	#[must_use]
	pub async fn generate_lockfile(&self, path: &Path, locked: bool) -> Result<()> {
		// Open the manifest file.
		let manifest_path = path.join("tangram.json");
		let mut manifest_file = tokio::fs::File::open(&manifest_path)
			.await
			.with_context(|| {
				format!(
					r#"Failed to open the package manifest at path "{}"."#,
					manifest_path.display()
				)
			})?;

		// Acquire a lock on the manifest file to detect a cyclic dependency or concurrent lockfile generation.
		let manifest_lock = Lock::new(&manifest_path, ());
		let manifest_lock_result = manifest_lock
			.try_lock_exclusive()
			.await
			.context("Failed to acquire a lock on the manifest.")?;
		let _manifest_lock_guard = match manifest_lock_result {
			Some(guard) => guard,
			None => {
				bail!("Encountered a cyclic dependency or concurrent lockfile generation.")
			},
		};

		// Read the manifest.
		let mut manifest = String::new();
		manifest_file
			.read_to_string(&mut manifest)
			.await
			.context("Failed to read the package manifest.")?;

		// Deserialize the manifest.
		let manifest: Manifest = serde_json::from_str(&manifest).with_context(|| {
			format!(
				r#"Failed to deserialize the package manifest at path "{}"."#,
				manifest_path.display()
			)
		})?;

		// Get the dependencies.
		let mut dependencies = BTreeMap::new();
		for (dependency_name, dependency) in manifest.dependencies.unwrap_or_default() {
			// Get the path dependency.
			let entry = match dependency {
				crate::manifest::Dependency::PathDependency(dependency) => {
					// Get the absolute path to the dependency.
					let dependency_path = path.join(&dependency.path);
					let dependency_path = tokio::fs::canonicalize(&dependency_path)
						.await
						.with_context(|| {
							format!("Could not canonicalize \"{}\"", dependency_path.display())
						})?;

					// Get the dependency's hash.
					let dependency_package_hash = self
						.checkin_package(&dependency_path, locked)
						.await
						.context("Failed to check in the dependency.")?;

					// Get the dependency package.
					let dependency_package = self.get_package_local(dependency_package_hash)?;

					// Create the lockfile entry.
					lockfile::Dependency {
						hash: dependency_package_hash,
						source: dependency_package.source,
						dependencies: None,
					}
				},

				// Handle a registry dependency.
				crate::manifest::Dependency::RegistryDependency(dependency) => {
					// Get the package hash from the registry.
					let dependency_version = &dependency.version;
					let dependency_source_hash = self.inner.api_client
						.get_package_version(&dependency_name, &dependency.version)
						.await
						.with_context(||
							format!(r#"Package with name "{dependency_name}" and version "{dependency_version}" is not in the package registry."#)
						)?;

					// Create a client.
					let client = self.create_client(self.inner.api_client.url.clone(), None);

					// Pull the source.
					self.pull(&client, dependency_source_hash)
						.await
						.context("Failed to pull.")?;

					// Checkin the package we just pulled.
					let dependency_package_hash = self
						.checkin_package_from_artifact_hash(dependency_source_hash)
						.await?;

					// Create the lockfile Entry.
					lockfile::Dependency {
						hash: dependency_package_hash,
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
}
