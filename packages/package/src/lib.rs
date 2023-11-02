pub use self::specifier::Specifier;
use async_recursion::async_recursion;
use std::collections::{BTreeMap, HashSet, VecDeque};
use tangram_client as tg;
use tangram_lsp::Module;
use tg::{Result, Subpath, WrapErr};

pub mod specifier;

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
	let mut queue: VecDeque<Subpath> = VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
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
				let dependency = match dependency {
					tg::Dependency::Path(dependency_path) => tg::Dependency::Path(
						module_subpath
							.clone()
							.into_relpath()
							.parent()
							.join(dependency_path.clone()),
					),
					tg::Dependency::Registry(_) => dependency.clone(),
				};

				// Get the dependency package.
				let tg::Dependency::Path(dependency_relpath) = &dependency else {
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
