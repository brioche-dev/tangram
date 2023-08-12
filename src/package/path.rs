use super::{
	lockfile::{self, Lockfile},
	Dependency, Package, LOCKFILE_FILE_NAME, ROOT_MODULE_FILE_NAME,
};
use crate::{
	artifact::Artifact,
	blob::Blob,
	block::Block,
	directory,
	error::{Result, WrapErr},
	file::File,
	instance::Instance,
	module::{self, Module},
	path::Subpath,
};
use async_recursion::async_recursion;
use std::{
	collections::{BTreeMap, HashSet, VecDeque},
	path::Path,
};

impl Package {
	/// Create a package from a path.
	#[async_recursion]
	pub async fn with_path(tg: &Instance, package_path: &Path) -> Result<Self> {
		// Create a builder for the directory.
		let mut directory = directory::Builder::new();

		// Create the dependencies map.
		let mut dependency_packages: Vec<Package> = Vec::new();
		let mut dependencies: BTreeMap<Dependency, Block> = BTreeMap::default();

		// Create a queue of module paths to visit and a visited set.
		let mut queue: VecDeque<Subpath> =
			VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
		let mut visited: HashSet<Subpath, fnv::FnvBuildHasher> = HashSet::default();

		// Add each module and its includes to the directory.
		while let Some(module_subpath) = queue.pop_front() {
			// Get the module's path.
			let module_path = package_path.join(module_subpath.to_string());

			// Add the module to the package directory.
			let artifact = Artifact::check_in(tg, &module_path).await?;
			directory = directory.add(tg, &module_subpath, artifact).await?;

			// Get the module's text.
			let permit = tg.file_descriptor_semaphore.acquire().await;
			let text = tokio::fs::read_to_string(&module_path)
				.await
				.wrap_err("Failed to read the module.")?;
			drop(permit);

			// Analyze the module.
			let analyze_output = Module::analyze(text).wrap_err("Failed to analyze the module.")?;

			// Add the includes to the package directory.
			for include_path in analyze_output.includes {
				// Get the included artifact's path in the package.
				let included_artifact_subpath = module_subpath
					.clone()
					.into_relpath()
					.parent()
					.join(include_path.clone())
					.try_into_subpath()
					.wrap_err("Invalid include path.")?;

				// Get the included artifact's path.
				let included_artifact_path =
					package_path.join(included_artifact_subpath.to_string());

				// Check in the artifact at the included path.
				let included_artifact = Artifact::check_in(tg, &included_artifact_path).await?;

				// Add the included artifact to the directory.
				directory = directory
					.add(tg, &included_artifact_subpath, included_artifact)
					.await?;
			}

			// Recurse into the dependencies.
			for import in &analyze_output.imports {
				if let module::Import::Dependency(dependency) = import {
					// Ignore duplicate dependencies.
					if dependencies.contains_key(dependency) {
						continue;
					}

					// Convert the module dependency to a package dependency.
					let dependency = match dependency {
						Dependency::Path(dependency_path) => Dependency::Path(
							module_subpath
								.clone()
								.into_relpath()
								.parent()
								.join(dependency_path.clone()),
						),
						Dependency::Registry(_) => dependency.clone(),
					};

					// Get the dependency package.
					let Dependency::Path(dependency_relpath) = &dependency else {
						unimplemented!();
					};
					let dependency_package_path = package_path.join(dependency_relpath.to_string());
					let dependency_package = Self::with_path(tg, &dependency_package_path).await?;

					// Add the dependency.
					dependencies.insert(dependency.clone(), dependency_package.block().clone());
					dependency_packages.push(dependency_package);
				}
			}

			// Add the module subpath to the visited set.
			visited.insert(module_subpath.clone());

			// Add the unvisited path imports to the queue.
			for import in &analyze_output.imports {
				if let module::Import::Path(import) = import {
					let imported_module_subpath = module_subpath
						.clone()
						.into_relpath()
						.parent()
						.join(import.clone())
						.try_into_subpath()
						.wrap_err("Failed to resolve the module path.")?;
					if !visited.contains(&imported_module_subpath) {
						queue.push_back(imported_module_subpath);
					}
				}
			}
		}

		// Create the lockfile.
		let lockfile_dependencies = dependencies
			.iter()
			.map(|(dependency, block)| (dependency.clone(), lockfile::Entry::Locked(block.id())))
			.collect();
		let references = dependency_packages
			.into_iter()
			.map(|package| package.artifact)
			.collect();
		let lockfile = Lockfile {
			dependencies: lockfile_dependencies,
		};
		let lockfile = serde_json::to_vec(&lockfile).unwrap();
		let lockfile = Blob::with_bytes(tg, &lockfile).await?;
		let lockfile = File::builder(lockfile)
			.references(references)
			.build(tg)
			.await?;
		let lockfile_subpath = LOCKFILE_FILE_NAME.parse().unwrap();
		directory = directory.add(tg, &lockfile_subpath, lockfile).await?;

		// Create the package directory.
		let directory = directory.build().await?;

		// Create the package.
		let package = Self {
			artifact: directory.into(),
			dependencies: Some(dependencies),
		};

		Ok(package)
	}
}
