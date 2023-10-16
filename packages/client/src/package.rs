pub use self::{dependency::Dependency, specifier::Specifier};
use crate::{
	artifact, directory,
	module::{self, Import},
	object, return_error, Artifact, Client, Module, Result, Subpath, WrapErr,
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

crate::id!(Package);
crate::handle!(Package);

#[derive(Clone, Copy, Debug)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Package(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub artifact: Artifact,
	pub dependencies: BTreeMap<Dependency, Package>,
}

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
	pub dependencies: BTreeMap<Dependency, Id>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Registry {
	pub name: String,
	pub versions: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchResult {
	pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
	pub name: Option<String>,
	pub version: Option<String>,
}

impl Package {
	pub async fn with_specifier(client: &dyn Client, specifier: Specifier) -> Result<Self> {
		match specifier {
			Specifier::Path(path) => Ok(Self::with_path(client, &path).await?),
			Specifier::Registry(_) => unimplemented!(),
		}
	}

	/// Create a package from a path.
	#[async_recursion]
	pub async fn with_path(client: &dyn Client, package_path: &Path) -> Result<Self> {
		if client.is_local() {
			if let Some(package) = client.try_get_package_for_path(package_path).await? {
				return Ok(package);
			}
		}

		// Create a builder for the directory.
		let mut directory = directory::Builder::default();

		// Create the dependencies map.
		let mut dependency_packages: Vec<Self> = Vec::new();
		let mut dependencies: BTreeMap<Dependency, Self> = BTreeMap::default();

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
				if let Import::Dependency(dependency) = import {
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

		// Create the package directory.
		let directory = directory.build();

		// Create the package.
		let package = Self::with_object(Object {
			artifact: directory.into(),
			dependencies,
		});

		if client.is_local() {
			client
				.set_package_for_path(package_path, package.clone())
				.await?;
		}

		Ok(package)
	}

	pub async fn artifact(&self, client: &dyn Client) -> Result<&Artifact> {
		Ok(&self.object(client).await?.artifact)
	}

	pub async fn dependencies(&self, client: &dyn Client) -> Result<&BTreeMap<Dependency, Self>> {
		Ok(&self.object(client).await?.dependencies)
	}

	pub async fn root_module(&self, client: &dyn Client) -> Result<Module> {
		Ok(Module::Normal(module::Normal {
			package_id: self.id(client).await?,
			path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		}))
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let artifact = self.artifact.expect_id();
		let dependencies = self
			.dependencies
			.iter()
			.map(|(dependency, package)| (dependency.clone(), package.expect_id()))
			.collect();
		Data {
			artifact,
			dependencies,
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let artifact = Artifact::with_id(data.artifact);
		let dependencies = data
			.dependencies
			.into_iter()
			.map(|(dependency, id)| (dependency, Package::with_id(id)))
			.collect();
		Self {
			artifact,
			dependencies,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Handle> {
		std::iter::empty()
			.chain(
				self.dependencies
					.values()
					.cloned()
					.map(|package| package.handle().clone())
					.map(Into::into),
			)
			.chain(std::iter::once(self.artifact.handle().clone()))
			.collect()
	}
}

impl Data {
	pub fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
		if version != 0 {
			return_error!(r#"Cannot deserialize with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes)?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		vec![self.artifact.into()]
	}
}

pub mod dependency {
	pub use crate::package::specifier::Registry;
	use crate::{Error, Relpath, Result};

	/// A dependency on a package, either at a path or from the registry.
	#[derive(
		Clone,
		Debug,
		Eq,
		Hash,
		Ord,
		PartialEq,
		PartialOrd,
		serde::Deserialize,
		serde::Serialize,
		tangram_serialize::Deserialize,
		tangram_serialize::Serialize,
	)]
	#[serde(into = "String", try_from = "String")]
	#[tangram_serialize(into = "String", try_from = "String")]
	pub enum Dependency {
		/// A dependency on a package at a path.
		Path(Relpath),

		/// A dependency on a package from the registry.
		Registry(Registry),
	}

	impl std::fmt::Display for Dependency {
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			match self {
				Dependency::Path(path) => {
					write!(f, "{path}")?;
					Ok(())
				},

				Dependency::Registry(registry) => {
					write!(f, "{registry}")?;
					Ok(())
				},
			}
		}
	}

	impl std::str::FromStr for Dependency {
		type Err = Error;

		fn from_str(value: &str) -> Result<Dependency> {
			if value.starts_with('.') {
				// If the string starts with `.`, then parse the string as a relative path.
				let path = value.parse()?;
				Ok(Dependency::Path(path))
			} else {
				// Otherwise, parse the string as a registry dependency.
				let registry = value.parse()?;
				Ok(Dependency::Registry(registry))
			}
		}
	}

	impl TryFrom<String> for Dependency {
		type Error = Error;

		fn try_from(value: String) -> Result<Self, Self::Error> {
			value.parse()
		}
	}

	impl From<Dependency> for String {
		fn from(value: Dependency) -> Self {
			value.to_string()
		}
	}
}

pub mod specifier {
	use super::dependency;
	use crate::{Error, Result};
	use std::path::PathBuf;

	/// A reference to a package, either at a path or from the registry.
	#[derive(
		Clone,
		Debug,
		Eq,
		Ord,
		PartialEq,
		PartialOrd,
		serde::Deserialize,
		serde::Serialize,
		tangram_serialize::Deserialize,
		tangram_serialize::Serialize,
	)]
	#[serde(into = "String", try_from = "String")]
	#[tangram_serialize(into = "String", try_from = "String")]
	pub enum Specifier {
		/// A reference to a package at a path.
		Path(PathBuf),

		/// A reference to a package from the registry.
		Registry(Registry),
	}

	#[derive(
		Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
	)]
	pub struct Registry {
		/// The name.
		name: String,

		/// The version.
		version: Option<String>,
	}

	impl std::fmt::Display for Specifier {
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			match self {
				Specifier::Path(path) => {
					let path = path.display();
					write!(f, "{path}")?;
					Ok(())
				},

				Specifier::Registry(specifier) => {
					write!(f, "{specifier}")?;
					Ok(())
				},
			}
		}
	}

	impl std::str::FromStr for Specifier {
		type Err = Error;

		fn from_str(value: &str) -> Result<Specifier> {
			if value.starts_with('/') || value.starts_with('.') {
				// If the string starts with `/` or `.`, then parse the string as a path.
				let specifier = value.parse()?;
				Ok(Specifier::Path(specifier))
			} else {
				// Otherwise, parse the string as a registry specifier.
				let specifier = value.parse()?;
				Ok(Specifier::Registry(specifier))
			}
		}
	}

	impl From<Specifier> for String {
		fn from(value: Specifier) -> Self {
			value.to_string()
		}
	}

	impl TryFrom<String> for Specifier {
		type Error = Error;

		fn try_from(value: String) -> Result<Self, Self::Error> {
			value.parse()
		}
	}

	impl std::fmt::Display for Registry {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			let name = &self.name;
			write!(f, "{name}")?;
			if let Some(version) = &self.version {
				write!(f, "@{version}")?;
			}
			Ok(())
		}
	}

	impl std::str::FromStr for Registry {
		type Err = Error;

		fn from_str(value: &str) -> Result<Registry> {
			let mut components = value.split('@');
			let name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);
			Ok(Registry { name, version })
		}
	}

	impl From<Registry> for String {
		fn from(value: Registry) -> Self {
			value.to_string()
		}
	}

	impl TryFrom<String> for Registry {
		type Error = Error;

		fn try_from(value: String) -> Result<Self, Self::Error> {
			value.parse()
		}
	}

	impl From<dependency::Dependency> for Specifier {
		fn from(value: dependency::Dependency) -> Self {
			match value {
				dependency::Dependency::Path(path) => Specifier::Path(path.into()),
				dependency::Dependency::Registry(specifier) => Specifier::Registry(specifier),
			}
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;

		#[test]
		fn test() {
			let left: Specifier = "hello".parse().unwrap();
			let right = Specifier::Registry(Registry {
				name: "hello".to_owned(),
				version: None,
			});
			assert_eq!(left, right);

			let left: Specifier = "hello@0.0.0".parse().unwrap();
			let right = Specifier::Registry(Registry {
				name: "hello".to_owned(),
				version: Some("0.0.0".to_owned()),
			});
			assert_eq!(left, right);

			let path_specifiers = [".", "./", "./hello"];
			for path_specifier in path_specifiers {
				let left: Specifier = path_specifier.parse().unwrap();
				let right = Specifier::Path(PathBuf::from(path_specifier));
				assert_eq!(left, right);
			}
		}
	}
}
