use async_trait::async_trait;
use std::{
	collections::{BTreeMap, HashSet, VecDeque},
	path::Path,
};
use tangram_client as tg;
use tangram_lsp::Module;
use tg::{return_error, Result, Subpath, WrapErr};

#[async_trait]
pub trait PackageExt {
	async fn with_specifier(
		client: &dyn tg::Client,
		specifier: tg::package::Specifier,
	) -> Result<tg::Package>;

	async fn with_path(client: &dyn tg::Client, package_path: &Path) -> Result<tg::Package>;

	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata>;

	async fn root_module(&self, client: &dyn tg::Client) -> Result<Module>;
}

#[async_trait]
impl PackageExt for tg::Package {
	async fn with_specifier(
		client: &dyn tg::Client,
		specifier: tg::package::Specifier,
	) -> Result<tg::Package> {
		match specifier {
			tg::package::Specifier::Path(path) => Ok(Self::with_path(client, &path).await?),
			tg::package::Specifier::Registry(_) => unimplemented!(),
		}
	}

	async fn metadata(&self, client: &dyn tg::Client) -> Result<tg::package::Metadata> {
		let module = self.root_module(client).await?.unwrap_normal();
		let directory = self
			.artifact(client)
			.await?
			.clone()
			.try_unwrap_directory()
			.unwrap();
		let file = directory
			.get(client, &module.path)
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

	/// Create a package from a path.
	async fn with_path(client: &dyn tg::Client, package_path: &Path) -> Result<tg::Package> {
		// if client.is_local() {
		// 	if let Some(package) = client.try_get_package_for_path(package_path).await? {
		// 		return Ok(package);
		// 	}
		// }

		// Create a builder for the directory.
		let mut directory = tg::directory::Builder::default();

		// Create the dependencies map.
		let mut dependency_packages: Vec<Self> = Vec::new();
		let mut dependencies: BTreeMap<tg::package::Dependency, Self> = BTreeMap::default();

		// Create a queue of module paths to visit and a visited set.
		let mut queue: VecDeque<Subpath> =
			VecDeque::from(vec![tg::package::ROOT_MODULE_FILE_NAME.parse().unwrap()]);
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
				let included_artifact_path =
					package_path.join(included_artifact_subpath.to_string());

				// Check in the artifact at the included path.
				let included_artifact =
					tg::Artifact::check_in(client, &included_artifact_path).await?;

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
						tg::package::Dependency::Path(dependency_path) => {
							tg::package::Dependency::Path(
								module_subpath
									.clone()
									.into_relpath()
									.parent()
									.join(dependency_path.clone()),
							)
						},
						tg::package::Dependency::Registry(_) => dependency.clone(),
					};

					// Get the dependency package.
					let tg::package::Dependency::Path(dependency_relpath) = &dependency else {
						unimplemented!();
					};
					let dependency_package_path = package_path.join(dependency_relpath.to_string());
					let dependency_package =
						Self::with_path(client, &dependency_package_path).await?;

					// Add the dependency.
					dependencies.insert(dependency.clone(), dependency_package.clone());
					dependency_packages.push(dependency_package);
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

		// Create the package.
		let package = Self::with_object(tg::package::Object {
			artifact: directory.into(),
			dependencies,
		});

		// if client.is_local() {
		// 	client
		// 		.set_package_for_path(package_path, package.clone())
		// 		.await?;
		// }

		Ok(package)
	}

	async fn root_module(&self, client: &dyn tg::Client) -> Result<Module> {
		{
			Ok(Module::Normal(tangram_lsp::module::Normal {
				package_id: self.id(client).await?.clone(),
				path: tg::package::ROOT_MODULE_FILE_NAME.parse().unwrap(),
			}))
		}
	}
}
