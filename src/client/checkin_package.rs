use crate::{
	artifact::Artifact,
	client::Client,
	lockfile::{self, Lockfile},
	manifest::Manifest,
};
use anyhow::{anyhow, Result};
use std::{
	collections::{BTreeMap, HashMap, VecDeque},
	path::{Path, PathBuf},
};

impl Client {
	/// Checkin a package along with all its path dependencies.
	pub async fn checkin_package(&self, path: &Path) -> Result<Artifact> {
		let path = tokio::fs::canonicalize(path).await?;

		// Collect all path dependencies in topological order.
		let mut queue: VecDeque<PathBuf> = VecDeque::from(vec![path.to_owned()]);
		let mut package_paths: Vec<PathBuf> = Vec::new();
		let mut artifact_for_package_path: HashMap<PathBuf, Artifact> = HashMap::new();
		while let Some(package_path) = queue.pop_front() {
			// Add the path to the list of package paths.
			package_paths.push(package_path.clone());

			// Read the manifest.
			let manifest_path = package_path.join("tangram.json");
			let manifest = tokio::fs::read(&manifest_path).await?;
			let manifest: Manifest = serde_json::from_slice(&manifest)?;

			// Add the package's path dependencies to the queue.
			if let Some(dependencies) = manifest.dependencies {
				for dependency in dependencies.values() {
					match dependency {
						crate::manifest::Dependency::PathDependency(dependency) => {
							let dependency_path = package_path.join(&dependency.path);
							let dependency_path = tokio::fs::canonicalize(&dependency_path).await?;
							queue.push_back(dependency_path);
						},
						crate::manifest::Dependency::RegistryDependency(_) => continue,
					}
				}
			}
		}

		// Write the lockfile for each package and check it in.
		package_paths.reverse();
		let mut root_package_artifact = None;
		for package_path in package_paths {
			// If this package has already been checked in, then continue.
			if artifact_for_package_path.get(&package_path).is_some() {
				continue;
			}

			// Read the manifest.
			let manifest_path = package_path.join("tangram.json");
			let manifest = tokio::fs::read(&manifest_path).await?;
			let manifest: Manifest = serde_json::from_slice(&manifest)?;

			// Create the lockfile.
			let mut lockfile = Lockfile(BTreeMap::new());
			for (dependency_name, dependency) in manifest.dependencies.iter().flatten() {
				// Retrieve the path dependency.
				let dependency = match dependency {
					crate::manifest::Dependency::PathDependency(dependency) => dependency,
					crate::manifest::Dependency::RegistryDependency(_) => continue,
				};

				// Get the absolute path to the dependency.
				let dependency_path = package_path.join(&dependency.path);
				let dependency_path = tokio::fs::canonicalize(&dependency_path).await?;

				// Get the artifact for the dependency.
				let dependency_artifact = artifact_for_package_path
					.get(&dependency_path)
					.ok_or_else(|| {
						anyhow!(
							r#"Failed to get the artifact for path "{}"."#,
							dependency_path.display(),
						)
					})?
					.clone();

				// Add a lockfile entry for the dependency.
				let entry = lockfile::Entry {
					package: dependency_artifact.object_hash,
					dependencies: Lockfile(BTreeMap::new()),
				};
				lockfile.0.insert(dependency_name.to_owned(), entry);
			}

			// Write the lockfile.
			let lockfile = serde_json::to_vec_pretty(&lockfile)?;
			let lockfile_path = package_path.join("tangram.lock");
			tokio::fs::write(lockfile_path, lockfile).await?;

			// Check in the package.
			let artifact = self.checkin(&package_path).await?;
			artifact_for_package_path.insert(package_path, artifact.clone());
			root_package_artifact = Some(artifact);
		}

		Ok(root_package_artifact.unwrap())
	}
}
