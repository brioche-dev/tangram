pub use self::{dependency::Dependency, specifier::Specifier};
use crate::{artifact, object, Artifact, Client, Result, WrapErr};
use bytes::Bytes;
use std::collections::BTreeMap;

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

crate::id!(Package);
crate::handle!(Package);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Package(object::Handle);

#[derive(Clone, Debug)]
pub struct Object {
	pub artifact: Artifact,
	pub dependencies: BTreeMap<Dependency, Package>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Data {
	pub artifact: artifact::Id,
	pub dependencies: BTreeMap<Dependency, Id>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Metadata {
	pub name: Option<String>,
	pub version: Option<String>,
	pub description: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Registry {
	pub name: String,
	pub versions: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct SearchResult {
	pub name: String,
	pub description: String,
	pub latest_version: String,
	pub last_updated: u64,
}

impl Package {
	pub async fn artifact(&self, client: &dyn Client) -> Result<&Artifact> {
		Ok(&self.object(client).await?.artifact)
	}

	pub async fn dependencies(&self, client: &dyn Client) -> Result<&BTreeMap<Dependency, Self>> {
		Ok(&self.object(client).await?.dependencies)
	}
}

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let artifact = self.artifact.expect_id();
		let dependencies = self
			.dependencies
			.iter()
			.map(|(dependency, package)| (dependency.clone(), package.expect_id().clone()))
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
	pub fn serialize(&self) -> Result<Bytes> {
		serde_json::to_vec(self)
			.map(Into::into)
			.wrap_err("Failed to serialize the data.")
	}

	pub fn deserialize(bytes: &Bytes) -> Result<Self> {
		serde_json::from_reader(bytes.as_ref()).wrap_err("Failed to deserialize the data.")
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		vec![self.artifact.clone().into()]
	}
}

pub mod dependency {
	pub use crate::package::specifier::Registry;
	use crate::{Error, Relpath, Result};

	/// A dependency on a package, either at a path or from the registry.
	#[derive(
		Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
	)]
	#[serde(into = "String", try_from = "String")]
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
	use crate::{Error, Result, WrapErr};
	use std::path::PathBuf;

	/// A reference to a package, either at a path or from the registry.
	#[derive(
		Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
	)]
	#[serde(into = "String", try_from = "String")]
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
				let specifier = value.parse().wrap_err("Failed to parse the specifier.")?;
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
