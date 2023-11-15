use self::lockfile::Lockfile;
pub use self::specifier::Specifier;
use crate::{Import, Module};
use async_recursion::async_recursion;
use async_trait::async_trait;
use itertools::Itertools;
use std::{
	collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
	path::{Path, PathBuf},
};
use tangram_client as tg;
use tangram_error::{return_error, Result, WrapErr};
use tg::{package::Metadata, Dependency, Relpath, Subpath};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub mod lockfile;
pub mod specifier;

#[cfg(test)]
mod tests;
pub mod version;

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

pub struct Options {
	pub update: bool,
}

/// Look up the corresponding artifact and lock of a package for a given module path. If the lockfile cannot be found or the `imports_changed` flag is set "true", then a new lockfile is created. If the lockfile's dependencies for the root artifact are different than the dependencies in an existing lockfile, the lockfile is removed and we attempt to lock again.
pub async fn get_or_create(
	client: &dyn tg::Client,
	module_path: &Path,
) -> Result<(tg::Artifact, tg::Lock)> {
	// Find the package path for this module path.
	let mut package_path = module_path.to_owned();
	while !package_path.join(ROOT_MODULE_FILE_NAME).exists() {
		if !package_path.pop() {
			return_error!("Could not find root module.");
		}
	}

	let package_path = package_path
		.canonicalize()
		.wrap_err("Failed to canonicalize path.")?;

	// First, try and read from an existing lockfile.
	let lockfile_path = package_path.join(LOCKFILE_FILE_NAME);
	if lockfile_path.exists() {
		// Deserialize the lockfile.
		let mut file = tokio::fs::File::open(&lockfile_path)
			.await
			.wrap_err("Failed to open lockfile.")?;
		let mut contents = Vec::new();
		file.read_to_end(&mut contents)
			.await
			.wrap_err("Failed to read lockfile contents.")?;
		let lockfile: Lockfile =
			serde_json::from_slice(&contents).wrap_err("Failed to deserialize the lockfile.")?;

		// Get the root lock.
		let lock = lockfile.lock(&tg::Relpath::empty())?;

		// Scan the root artifact.
		let mut visited = BTreeMap::new();
		let mut path_dependencies = BTreeMap::new();
		let artifact = analyze_package_at_path(
			client,
			package_path.clone(),
			&mut visited,
			&mut path_dependencies,
		)
		.await?;

		// Verify that our dependencies all match.
		let current_dependencies = artifact.dependencies(client).await?;
		let locked_dependencies = lockfile
			.locks
			.get(lock.id(client).await?)
			.into_iter()
			.flatten()
			.map(|(k, _)| k);

		// If the dependencies are all the same, we can use the existing lockfile. Otherwise, fall through.
		if current_dependencies
			.iter()
			.zip(locked_dependencies)
			.all_equal()
		{
			return Ok((artifact.into(), lock));
		}
	}

	// Create the package, lock, and lockfile.
	let (artifact, lock, lockfile) = create(client, &Specifier::Path(package_path)).await?;

	// Write the lockfile to disk.
	let mut file = tokio::fs::File::options()
		.read(true)
		.write(true)
		.create(true)
		.append(false)
		.open(lockfile_path)
		.await
		.wrap_err("Failed to open lockfile for writing.")?;
	let contents =
		serde_json::to_vec_pretty(&lockfile).wrap_err("Failed to serialize lockfile.")?;
	file.write_all(&contents)
		.await
		.wrap_err("Failed to write lockfile.")?;

	// Return.
	Ok((artifact, lock))
}

pub async fn create(
	client: &dyn tg::Client,
	specifier: &Specifier,
) -> Result<(tg::Artifact, tg::Lock, Lockfile)> {
	let (root_artifact, path_dependencies) = match specifier {
		Specifier::Path(path) => {
			// Canonicalize.
			let package_path = path
				.canonicalize()
				.wrap_err("Failed to canonicalize path.")?;
			let mut visited = BTreeMap::new();
			let mut path_dependencies = BTreeMap::new();
			let root_artifact =
				analyze_package_at_path(client, package_path, &mut visited, &mut path_dependencies)
					.await?;
			(root_artifact, path_dependencies)
		},
		Specifier::Registry(specifier::Registry { name, version }) => {
			let Some(version) = version else {
				return_error!("Cannot create package from registry dependency without a version.");
			};
			let id =
				client
					.get_package_version(name, version)
					.await?
					.ok_or(tangram_error::error!(
						"Could not find package {name}@{version}."
					))?;
			let root_artifact = tg::Artifact::with_id(id)
				.try_unwrap_directory()
				.wrap_err("Expected package artifact to be a directory.")?;
			let path_dependencies = BTreeMap::new();
			(root_artifact, path_dependencies)
		},
	};

	// Solve the version constraints.
	let root = root_artifact.id(client).await?.clone().into();
	let paths = version::solve(client, root, path_dependencies).await?;

	// Get the root lock and create a lockfile.
	let root_lock = paths[0].1.clone();
	let lockfile = Lockfile::with_paths(client, paths).await?;
	Ok((root_artifact.into(), root_lock, lockfile))
}

#[async_trait]
impl Ext for tg::Directory {
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
				if let Import::Dependency(dependency) = import {
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
				if let Import::Path(import) = import {
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
pub trait Ext {
	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata>;
	async fn dependencies(&self, client: &dyn tg::Client) -> Result<Vec<tg::Dependency>>;
}

// Recursively check in a package, its includes, and path dependencies. Return the directory artifact of the root package, and fill the path_dependencies table.
#[async_recursion]
async fn analyze_package_at_path(
	client: &dyn tg::Client,
	package_path: PathBuf,
	visited: &mut BTreeMap<PathBuf, Option<tg::Directory>>,
	path_dependencies: &mut BTreeMap<tg::Id, BTreeMap<Relpath, tg::Id>>,
) -> tangram_error::Result<tg::Directory> {
	debug_assert!(
		package_path.is_absolute(),
		"Expected an absolute path, got {package_path:#?}."
	);
	match visited.get(&package_path) {
		Some(Some(directory)) => return Ok(directory.clone()),
		Some(None) => return Err(tangram_error::error!("Cyclical path dependencies found.")),
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
				Import::Dependency(dependency) if dependency.path.is_some() => {
					let dependency_path = dependency.path.as_ref().unwrap();
					let package_path = module_path
						.parent()
						.unwrap()
						.join(dependency_path.to_string())
						.canonicalize()
						.wrap_err("Failed to canonicalize dependency path.")?;

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
			if let Import::Path(import) = import {
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
	pub async fn new(
		client: &dyn tg::Client,
		artifact: tg::Artifact,
	) -> tangram_error::Result<Self> {
		let id = artifact
			.id(client)
			.await
			.wrap_err("Failed to get package ID.")?
			.into();
		let metadata = client
			.get_package_metadata(&id)
			.await?
			.ok_or(tangram_error::error!("Missing package metadata."))?;
		let dependencies = client
			.get_package_dependencies(&id)
			.await?
			.unwrap_or_default();
		Ok(Self {
			metadata,
			dependencies,
		})
	}

	pub fn name(&self) -> tangram_error::Result<&str> {
		self.metadata
			.name
			.as_deref()
			.ok_or(tangram_error::error!("Missing package name."))
	}

	pub fn version(&self) -> tangram_error::Result<&str> {
		self.metadata
			.version
			.as_deref()
			.ok_or(tangram_error::error!("Missing package version."))
	}

	pub fn registry_dependencies(&self) -> impl Iterator<Item = &'_ tg::Dependency> {
		self.dependencies
			.iter()
			.filter(|dependency| dependency.path.is_none())
	}
}
