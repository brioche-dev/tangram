pub use self::specifier::Specifier;
use async_recursion::async_recursion;
use async_trait::async_trait;
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};
use std::{
	collections::{BTreeMap, HashSet, VecDeque},
	path::PathBuf,
};
use tangram_client as tg;
use tangram_error::{return_error, Result, WrapErr};
use tangram_lsp::Module;

pub mod lockfile;
pub mod specifier;
pub mod version;
mod tests;

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

// Create a package.
#[async_recursion]
pub async fn new(
	client: &dyn tg::Client,
	specifier: &Specifier,
) -> Result<(tg::Artifact, tg::Lock)> {
	let package_path = match specifier {
		Specifier::Path(path) => path,
		Specifier::Registry(_) => unimplemented!(),
	};

	// Create a builder for the directory.
	let mut directory = tg::directory::Builder::default();

	// Create the dependencies map.
	let mut dependencies: BTreeMap<tg::Dependency, tg::lock::Entry> = BTreeMap::default();

	// Create a queue of module paths to visit and a visited set.
	let mut queue: VecDeque<tg::Subpath> =
		VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
	let mut visited: HashSet<tg::Subpath, fnv::FnvBuildHasher> = HashSet::default();

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
			if let tangram_lsp::Import::Dependency(dependency) = import {
				// Ignore duplicate dependencies.
				if dependencies.contains_key(dependency) {
					continue;
				}

				// Convert the module dependency to a package dependency.
				let dependency = match &dependency.path {
					Some(dependency_path) => tg::Dependency::with_path(
						module_subpath
							.clone()
							.into_relpath()
							.parent()
							.join(dependency_path.clone()),
					),
					None => dependency.clone(),
				};

				// Get the dependency package.
				let Some(dependency_relpath) = &dependency.path else {
					unimplemented!();
				};

				let dependency_package_path = package_path.join(dependency_relpath.to_string());
				let (dependency_package, dependency_lock) =
					new(client, &Specifier::Path(dependency_package_path.clone())).await?;

				// Add the dependency.
				dependencies.insert(
					dependency.clone(),
					tg::lock::Entry {
						package: dependency_package,
						lock: dependency_lock,
					},
				);
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

	// Create the package directory.
	let directory = directory.build();

	// Create the lock.
	let lock = tg::Lock::with_object(tg::lock::Object { dependencies });

	Ok((directory.into(), lock))
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

		let dependencies = dependencies
			.into_iter()
			.map(|dependency| match &dependency.path {
				Some(_) => unimplemented!(),
				None => dependency,
			})
			.collect::<Vec<_>>();

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

pub async fn new2(
	client: &dyn tg::Client,
	specifier: &Specifier,
) -> Result<(tg::Artifact, tg::Lock)> {
	let package_path = match specifier {
		Specifier::Path(path) => path,
		Specifier::Registry(_) => {
			unimplemented!("Creating locks for registry dependencies is unsupported.")
		},
	};

	// Collect the list of package roots.
	let mut roots = Vec::new();
	let mut visited = BTreeMap::new();
	scan(client, package_path.clone(), &mut visited, &mut roots).await?;

	// Now we have all of our roots we can solve.

	todo!()
}

// Recursively scan packages and their path dependencies.
#[async_recursion]
async fn scan(
	client: &dyn tg::Client,
	package_path: PathBuf,
	visited: &mut BTreeMap<PathBuf, bool>,
	roots: &mut Vec<(tg::Directory, Vec<tg::dependency::Registry>)>,
) -> tg::Result<()> {
	debug_assert!(package_path.is_absolute());
	match visited.get(&package_path) {
		Some(true) => return Ok(()),
		Some(false) => return Err(tg::error!("Cyclical path dependencies found.")),
		None => (),
	}
	visited.insert(package_path.clone(), false);

	// Create a builder for the directory.
	let mut directory = tg::directory::Builder::default();

	// Create the dependencies vec.
	let mut dependencies = Vec::new();

	// Create a queue of module paths to visit and a visited set.
	let mut queue: VecDeque<Subpath> = VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
	let mut visited_modules: HashSet<tg::Subpath, fnv::FnvBuildHasher> = HashSet::default();

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
				tangram_lsp::Import::Dependency(tg::Dependency::Path(dependency)) => {
					// recurse
					let package_path = package_path
						.join(dependency.to_string())
						.canonicalize()
						.wrap_err("Failed to canonicalize path.")?;
					scan(client, package_path, visited, roots).await?;
				},
				tangram_lsp::Import::Dependency(tg::Dependency::Registry(dependency)) => {
					dependencies.push(dependency.clone());
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
	let _ = visited.insert(package_path, true);

	let artifact = directory.build();
	roots.push((artifact, dependencies));

	Ok(())
}

// Get the dependencies from a package artifact.
async fn scan_direct_dependencies(
	client: &dyn tg::Client,
	package: tg::Directory,
) -> tg::Result<Vec<tg::Dependency>> {
	// Create the dependencies vec.
	let mut dependencies = Vec::new();

	// Create a queue of module paths to visit and a visited set.
	let mut queue: VecDeque<Subpath> = VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
	let mut visited_modules: HashSet<tg::Subpath, fnv::FnvBuildHasher> = HashSet::default();

	// Add each module and its includes to the directory.
	while let Some(module_subpath) = queue.pop_front() {
		// Get the module's path.
		// Add the module to the package directory.
		let artifact = package
			.get(client, &module_subpath)
			.await?
			.try_unwrap_file()
			.wrap_err("Expected a file.")?;

		// Get the module's text.
		let text = artifact.contents(client).await?.text(client).await?;

		// Analyze the module.
		let analyze_output = Module::analyze(text).wrap_err("Failed to analyze the module.")?;

		// Add dependencies, recursing if necessary.
		for import in &analyze_output.imports {
			match import {
				tangram_lsp::Import::Dependency(dependency) => {
					dependencies.push(dependency.clone());
				},
				tangram_lsp::Import::Path(path) => {
					queue.push_back(path.subpath().clone());
				},
			}
		}

		// Add the module subpath to the visited set.
		visited_modules.insert(module_subpath.clone());
	}

	Ok(dependencies)
}

// 	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata> {
// 		let module = self.root_module(client).await?.unwrap_normal();
// 		let directory = self
// 			.artifact(client)
// 			.await?
// 			.clone()
// 			.try_unwrap_directory()
// 			.unwrap();
// 		let file = directory
// 			.get(client, &module.path)
// 			.await?
// 			.try_unwrap_file()
// 			.unwrap();
// 		let text = file.contents(client).await?.text(client).await?;
// 		let output = Module::analyze(text)?;
// 		if let Some(metadata) = output.metadata {
// 			Ok(metadata)
// 		} else {
// 			return_error!("Missing package metadata.")
// 		}
// 	}
>>>>>>> 9663340 (WIP: port version solving code to tangram_package.)
