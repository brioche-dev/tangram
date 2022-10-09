use crate::{
	api_client::ApiClient,
	builder::Shared,
	expression::{Expression, Package},
	hash::Hash,
	lockfile::{self, Lockfile},
	manifest::Manifest,
};
use anyhow::{Context, Result};
use fnv::FnvBuildHasher;
use std::{
	collections::{BTreeMap, HashMap, VecDeque},
	path::{Path, PathBuf},
};

impl Shared {
	/// Check in a package from the provided source path.
	#[allow(clippy::too_many_lines)]
	pub async fn checkin_package(
		&self,
		api_client: &ApiClient,
		source_path: &Path,
		locked: bool,
	) -> Result<Hash> {
		let source_path = tokio::fs::canonicalize(source_path).await?;

		// Collect all path dependencies in topological order.
		let mut queue: VecDeque<PathBuf> = VecDeque::from(vec![source_path.clone()]);
		let mut package_source_paths: Vec<PathBuf> = Vec::new();
		while let Some(package_path) = queue.pop_front() {
			// Add the path to the list of package paths.
			package_source_paths.push(package_path.clone());

			// Read the manifest.
			let manifest_path = package_path.join("tangram.json");
			let manifest = tokio::fs::read(&manifest_path)
				.await
				.context("Failed to read the package manifest.")?;
			let manifest: Manifest = serde_json::from_slice(&manifest).with_context(|| {
				format!(
					r#"Failed to parse the package manifest at path "{}"."#,
					manifest_path.display()
				)
			})?;

			// Add the package's path dependencies to the queue.
			if let Some(dependencies) = manifest.dependencies {
				for dependency in dependencies.values() {
					match dependency {
						crate::manifest::Dependency::PathDependency(dependency) => {
							let dependency_path = package_path.join(&dependency.path);
							let dependency_path = tokio::fs::canonicalize(&dependency_path)
								.await
								.with_context(|| {
								format!(
									r#"Failed to canonicalize the dependency at path "{}"."#,
									dependency_path.display()
								)
							})?;
							queue.push_back(dependency_path);
						},
						crate::manifest::Dependency::RegistryDependency(_) => continue,
					}
				}
			}
		}

		// Reverse the package source paths to put them in reverse topological order.
		package_source_paths.reverse();

		// Write the lockfile for each package source, check it in, and create its package expression.
		let mut cache: HashMap<PathBuf, Hash, FnvBuildHasher> = HashMap::default();
		let mut root_package = None;
		for package_source_path in package_source_paths {
			// If this package has already been checked in, then continue.
			if cache.get(&package_source_path).is_some() {
				continue;
			}

			// Read the manifest.
			let manifest_path = package_source_path.join("tangram.json");
			let manifest = tokio::fs::read(&manifest_path).await?;
			let manifest: Manifest = serde_json::from_slice(&manifest)?;

			if !locked {
				// Create the lockfile for this package.
				let mut dependencies = BTreeMap::new();
				for (dependency_name, dependency) in manifest.dependencies.iter().flatten() {
					// Retrieve the path dependency.
					let entry = match dependency {
						crate::manifest::Dependency::PathDependency(dependency) => {
							// Get the absolute path to the dependency.
							let dependency_path = package_source_path.join(&dependency.path);
							let dependency_path = tokio::fs::canonicalize(&dependency_path).await?;

							// Get the dependency's expression hash.
							let dependency_hash =
								cache.get(&dependency_path).copied().with_context(|| {
									let dependency_path = dependency_path.display();
									format!(
										r#"Failed to get the artifact for path "{dependency_path}"."#
									)
								})?;

							// Get the dependency package.
							let dependency_package = self
								.get_expression(dependency_hash)
								.await?
								.into_package()
								.context("Hello")?;

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
							let package_hash = api_client
								.get_package_version(dependency_name, &dependency.version)
								.await
								.with_context(||
									format!(r#"Package with name "{dependency_name}" and version "{dependency_version}" is not in the package registry."#)
								)?;
							let package_source_hash = self.get_package_source(package_hash).await?;

							// Create the lockfile Entry.
							lockfile::Dependency {
								hash: package_hash,
								source: package_source_hash,
								dependencies: None,
							}
						},
					};

					// Add the dependency.
					dependencies.insert(dependency_name.clone(), entry);
				}

				// Write the lockfile.
				let lockfile = Lockfile::new_v1(dependencies);
				let lockfile = serde_json::to_vec_pretty(&lockfile)?;
				let lockfile_path = package_source_path.join("tangram.lock");
				tokio::fs::write(&lockfile_path, lockfile).await?;
			};

			// Check in the package source.
			let package_source_hash = self.checkin(&package_source_path).await?;

			// Read the lockfile.
			let lockfile_path = package_source_path.join("tangram.lock");
			let lockfile = tokio::fs::read(&lockfile_path).await?;
			let lockfile: Lockfile = serde_json::from_slice(&lockfile)?;

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
			let package_hash = self.add_expression(&Expression::Package(package)).await?;

			// Add the package to the cache.
			cache.insert(package_source_path.clone(), package_hash);

			root_package = Some(package_hash);
		}

		let root_package = root_package.unwrap();
		Ok(root_package)
	}

	pub async fn get_package_source(&self, package_hash: Hash) -> Result<Hash> {
		let package = self
			.get_expression(package_hash)
			.await?
			.into_package()
			.context("Expected package.")?;
		let package_source = package.source;
		Ok(package_source)
	}

	pub async fn get_package_manifest(&self, package_hash: Hash) -> Result<Manifest> {
		let package_source_hash = self.get_package_source(package_hash).await?;

		let source_artifact = self
			.get_expression(package_source_hash)
			.await?
			.into_artifact()
			.context("Expected an artifact.")?;

		let source_directory = self
			.get_expression(source_artifact.root)
			.await?
			.into_directory()
			.context("Expected a directory.")?;

		let manifest_hash = source_directory
			.entries
			.get("tangram.json")
			.copied()
			.context("The package source does not contain a manifest.")?;

		let manifest_blob_hash = self
			.get_expression(manifest_hash)
			.await?
			.as_file()
			.context("Expected the manifest to be a file.")?
			.blob;

		let manifest_path = self.get_blob(manifest_blob_hash).await?;

		let manifest = tokio::fs::read(&manifest_path)
			.await
			.context("Failed to read the package manifest.")?;

		let manifest: Manifest = serde_json::from_slice(&manifest)
			.context(r#"Failed to parse the package manifest."#)?;

		Ok(manifest)
	}
}
