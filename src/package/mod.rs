pub use self::{dependency::Dependency, specifier::Specifier};
use crate::{
	artifact, directory,
	error::{Result, WrapErr},
	module::{self, Module},
	subpath::Subpath,
	Artifact, Client, Package,
};
use async_recursion::async_recursion;
use std::{
	collections::{BTreeMap, HashSet, VecDeque},
	path::Path,
};

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

pub mod dependency;
pub mod specifier;

crate::id!(Package);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(Package);

#[derive(Clone, Debug)]
pub struct Value {
	pub artifact: Artifact,
	pub dependencies: Option<BTreeMap<Dependency, Handle>>,
}

crate::value!(Package);

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub artifact: artifact::Id,

	#[tangram_serialize(id = 1)]
	pub dependencies: Option<BTreeMap<Dependency, Id>>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
	pub name: Option<String>,
	pub version: Option<String>,
}

impl Handle {
	pub async fn with_specifier(client: &Client, specifier: Specifier) -> Result<Self> {
		match specifier {
			Specifier::Path(path) => Ok(Self::with_path(client, &path).await?),
			Specifier::Registry(_) => unimplemented!(),
		}
	}

	/// Create a package from a path.
	#[async_recursion]
	pub async fn with_path(client: &Client, package_path: &Path) -> Result<Self> {
		// Create a builder for the directory.
		let mut directory = directory::Builder::default();

		// Create the dependencies map.
		let mut dependency_packages: Vec<Self> = Vec::new();
		let mut dependencies: BTreeMap<Dependency, Package> = BTreeMap::default();

		// Create a queue of module paths to visit and a visited set.
		let mut queue: VecDeque<Subpath> =
			VecDeque::from(vec![ROOT_MODULE_FILE_NAME.parse().unwrap()]);
		let mut visited: HashSet<Subpath, fnv::FnvBuildHasher> = HashSet::default();

		// Add each module and its includes to the directory.
		while let Some(module_subpath) = queue.pop_front() {
			// Get the module's path.
			let module_path = package_path.join(module_subpath.to_string());

			// Add the module to the package directory.
			let artifact = Artifact::check_in(client, &module_path).await?;
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
				let included_artifact = Artifact::check_in(client, &included_artifact_path).await?;

				// Add the included artifact to the directory.
				directory = directory
					.add(client, &included_artifact_subpath, included_artifact)
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

		// Create the package directory.
		let directory = directory.build();

		// Create the package.
		let package = Handle::with_value(Value {
			artifact: directory.into(),
			dependencies: Some(dependencies),
		});

		Ok(package)
	}

	pub async fn artifact(&self, client: &Client) -> Result<&Artifact> {
		Ok(&self.value(client).await?.artifact)
	}

	pub async fn dependencies(
		&self,
		client: &Client,
	) -> Result<&Option<BTreeMap<Dependency, Package>>> {
		Ok(&self.value(client).await?.dependencies)
	}

	pub async fn root_module(&self, client: &Client) -> Result<Module> {
		Ok(Module::Normal(module::Normal {
			package: self.id(client).await?,
			path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		}))
	}
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let artifact = artifact::Handle::with_id(data.artifact);
		let dependencies = data.dependencies.map(|dependencies| {
			dependencies
				.into_iter()
				.map(|(dependency, id)| (dependency, Handle::with_id(id)))
				.collect()
		});
		Self {
			artifact,
			dependencies,
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		let artifact = self.artifact.expect_id();
		let dependencies = self.dependencies.as_ref().map(|dependencies| {
			dependencies
				.iter()
				.map(|(dependency, id)| (dependency.clone(), id.expect_id()))
				.collect()
		});
		Data {
			artifact,
			dependencies,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		let mut children = vec![];
		children.extend(
			self.dependencies
				.as_ref()
				.map(|dependencies| {
					dependencies
						.values()
						.cloned()
						.map(Into::into)
						.collect::<Vec<_>>()
				})
				.unwrap_or_default(),
		);
		children.push(self.artifact.clone().into());
		children
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		std::iter::once(self.artifact.into())
			.chain(
				self.dependencies
					.iter()
					.flatten()
					.map(|(_, id)| (*id).into()),
			)
			.collect()
	}
}
