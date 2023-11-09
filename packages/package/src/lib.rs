pub use self::specifier::Specifier;
use async_recursion::async_recursion;
use async_trait::async_trait;
use std::{
	collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
	path::PathBuf,
};
use tangram_client as tg;
use tangram_error::{return_error, Result, WrapErr};
use tangram_lsp::Module;
use tg::{package::Metadata, return_error, Dependency, Relpath, Result, Subpath, WrapErr};

pub mod lockfile;
pub mod specifier;
pub mod version;

#[cfg(test)]
mod tests;

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

pub async fn new(
	client: &dyn tg::Client,
	specifier: &Specifier,
) -> Result<(tg::Artifact, tg::Lock)> {
	let (root_artifact, path_dependencies) = match specifier {
		Specifier::Path(path) => {
			// Scan, checking in any the path dependencies and includes.
			let mut visited = BTreeMap::new();
			let mut path_dependencies = BTreeMap::new();
			let root_artifact =
				analyze_package_at_path(client, path.clone(), &mut visited, &mut path_dependencies)
					.await?;
			(root_artifact, path_dependencies)
		},
		Specifier::Registry(specifier::Registry { name, version }) if version.is_some() => {
			let version = version.as_deref().unwrap();
			let id = client
				.get_package_version(name, version)
				.await?
				.ok_or(tg::error!("Could not find package {name}@{version}."))?;
			let root_artifact = tg::Artifact::with_id(id)
				.try_unwrap_directory()
				.wrap_err("Expected package artifact to be a directory.")?;
			let path_dependencies = BTreeMap::new();
			(root_artifact, path_dependencies)
		},
		_ => {
			tg::return_error!("Creating locks for regsitry dependencies without a version specifiier is unsupported.");
		},
	};

	let root_id = root_artifact.id(client).await?.clone().into();

	// Now we have the root, we need to get its path overrides.
	let lock = version::solve(client, root_id, path_dependencies)
		.await?
		.map_err(|e| tg::error!("Failed to solve dependency versions. {e}"))?;

	Ok((root_artifact.into(), lock))
}

#[async_trait]
impl PackageExt for tg::Directory {
	async fn dependencies(&self, client: &dyn tg::Client) -> Result<Vec<tg::Dependency>> {
		// Create the dependencies map.
		let mut dependencies: BTreeSet<tg::Dependency> = BTreeSet::default();

		// Create a queue of module paths to visit and a visited set.
		let mut queue: VecDeque<tg::Subpath> =
			VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
		let mut visited: HashSet<tg::Subpath, fnv::FnvBuildHasher> = HashSet::default();

		// Add each dependency.
		while let Some(module_subpath) = queue.pop_front() {
			// Get the file.
			let file = self
				.get(client, &module_subpath.clone())
				.await?
				.try_unwrap_file()
				.unwrap();
			let text = file.contents(client).await?.text(client).await?;

			// Analyze the module.
			let analyze_output = Module::analyze(text).wrap_err("Failed to analyze the module.")?;

			// Recurse into the dependencies.
			for import in &analyze_output.imports {
				if let tangram_lsp::Import::Dependency(dependency) = import {
					// Ignore duplicate dependencies.
					if dependencies.contains(dependency) {
						continue;
					}
					dependencies.insert(dependency.clone());
				}
			}

			// Add the module subpath to the visited set.
			visited.insert(module_subpath.clone());

			// Add the unvisited path imports to the queue.
			for import in &analyze_output.imports {
				if let tangram_lsp::Import::Path(import) = import {
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

		let dependencies = dependencies.into_iter().collect::<Vec<_>>();

		Ok(dependencies.into_iter().collect::<Vec<_>>())
	}

	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata> {
		let file = self
			.get(client, &ROOT_MODULE_FILE_NAME.try_into().unwrap())
			.await?
			.try_unwrap_file()
			.unwrap();
		let text = file.contents(client).await?.text(client).await?;
		let output = Module::analyze(text)?;
		if let Some(metadata) = output.metadata {
			Ok(metadata)
		} else {
			return_error!("Missing package metadata.")
		}
	}
}

#[async_trait]
pub trait PackageExt {
	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata>;
	async fn dependencies(&self, client: &dyn tg::Client) -> Result<Vec<tg::Dependency>>;
}

// Recursively checkin a package, its includes, and path dependencies. Returns the directory artifact of the root package, and fills the path_dependencies table.
#[async_recursion]
async fn analyze_package_at_path(
	client: &dyn tg::Client,
	package_path: PathBuf,
	visited: &mut BTreeMap<PathBuf, Option<tg::Directory>>,
	path_dependencies: &mut BTreeMap<tg::Id, BTreeMap<Relpath, tg::Id>>,
) -> tg::Result<tg::Directory> {
	debug_assert!(package_path.is_absolute());
	match visited.get(&package_path) {
		Some(Some(directory)) => return Ok(directory.clone()),
		Some(None) => return Err(tg::error!("Cyclical path dependencies found.")),
		None => (),
	}
	visited.insert(package_path.clone(), None);

	// Create a builder for the directory.
	let mut directory = tg::directory::Builder::default();

	// Create a queue of module paths to visit and a visited set.
	let mut queue: VecDeque<Subpath> = VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
	let mut visited_modules: HashSet<tg::Subpath, fnv::FnvBuildHasher> = HashSet::default();

	let mut package_path_dependencies = BTreeMap::new();

	// Add each module and its includes to the directory.
	while let Some(module_subpath) = queue.pop_front() {
		// Get the module's path.
		let module_path = package_path.join(module_subpath.to_string());

		// Add the module to the package directory.
		let artifact = tg::Artifact::check_in(client, &module_path).await?;
		directory = directory.add(client, &module_subpath, artifact).await?;

		// Get the module's text.
		let permit = client.file_descriptor_semaphore().acquire().await;
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
			let included_artifact_path = package_path.join(included_artifact_subpath.to_string());

			// Check in the artifact at the included path.
			let included_artifact = tg::Artifact::check_in(client, &included_artifact_path).await?;

			// Add the included artifact to the directory.
			directory = directory
				.add(client, &included_artifact_subpath, included_artifact)
				.await?;
		}

		// Recurse into the dependencies.
		for import in &analyze_output.imports {
			match import {
				tangram_lsp::Import::Dependency(dependency) if dependency.path.is_some() => {
					// recurse
					let package_path = package_path
						.join(dependency.path.as_ref().unwrap().to_string())
						.canonicalize()
						.wrap_err("Failed to canonicalize path.")?;

					// This gives us a full directory ID.
					let child =
						analyze_package_at_path(client, package_path, visited, path_dependencies)
							.await?;
					let id = child.id(client).await?.clone();

					// Store the artifact and dependenc
					package_path_dependencies.insert(dependency.path.clone().unwrap(), id.into());
				},
				_ => (),
			}
		}

		// Add the module subpath to the visited set.
		visited_modules.insert(module_subpath.clone());

		// Add the unvisited path imports to the queue.
		for import in &analyze_output.imports {
			if let tangram_lsp::Import::Path(import) = import {
				let imported_module_subpath = module_subpath
					.clone()
					.into_relpath()
					.parent()
					.join(import.clone())
					.try_into_subpath()
					.wrap_err("Failed to resolve the module path.")?;
				if !visited_modules.contains(&imported_module_subpath) {
					queue.push_back(imported_module_subpath);
				}
			}
		}
	}

	// Create permanent mark.
	let artifact = directory.build();
	let id = artifact.id(client).await?.clone().into();
	let _ = visited.insert(package_path, Some(artifact.clone()));
	path_dependencies.insert(id, package_path_dependencies);
	Ok(artifact)
}

#[derive(Debug, Clone)]
pub struct Analysis {
	pub metadata: Metadata,
	pub dependencies: Vec<Dependency>,
}

impl Analysis {
	pub async fn new(client: &dyn tg::Client, artifact: tg::Artifact) -> tg::Result<Self> {
		let id = artifact
			.id(client)
			.await
			.wrap_err("Failed to get package ID.")?
			.into();
		let metadata = client
			.get_package_metadata(&id)
			.await?
			.ok_or(tg::error!("Missing package metadata."))?;
		let dependencies = client
			.get_package_dependencies(&id)
			.await?
			.unwrap_or_default();
		Ok(Self {
			metadata,
			dependencies,
		})
	}

	pub fn name(&self) -> tg::Result<&str> {
		self.metadata
			.name
			.as_deref()
			.ok_or(tg::error!("Missing package name."))
	}

	pub fn version(&self) -> tg::Result<&str> {
		self.metadata
			.version
			.as_deref()
			.ok_or(tg::error!("Missing package version."))
	}

	pub fn registry_dependencies(&self) -> impl Iterator<Item = &'_ tg::Dependency> {
		self.dependencies
			.iter()
			.filter(|dependency| dependency.path.is_none())
	}
}
