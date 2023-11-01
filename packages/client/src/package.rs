pub use self::{dependency::Dependency, specifier::Specifier};
use crate::{artifact, id, object, Artifact, Client, Error, Result, WrapErr};
use bytes::Bytes;
use derive_more::Display;
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::{collections::BTreeMap, sync::Arc};
use tangram_error::return_error;

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

#[derive(
	Clone,
	Debug,
	Display,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(into = "crate::Id", try_from = "crate::Id")]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Package {
	state: Arc<std::sync::RwLock<State>>,
}

type State = object::State<Id, Object>;

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

impl Id {
	pub fn new(bytes: &Bytes) -> Self {
		Self(crate::Id::new_hashed(id::Kind::Package, bytes))
	}

	#[must_use]
	pub fn to_bytes(&self) -> Bytes {
		self.0.to_bytes()
	}
}

impl Package {
	#[must_use]
	pub fn with_state(state: State) -> Self {
		Self {
			state: Arc::new(std::sync::RwLock::new(state)),
		}
	}

	#[must_use]
	pub fn state(&self) -> &std::sync::RwLock<State> {
		&self.state
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		let state = State::with_id(id);
		Self {
			state: Arc::new(std::sync::RwLock::new(state)),
		}
	}

	#[must_use]
	pub fn with_object(object: Object) -> Self {
		let state = State::with_object(object);
		Self {
			state: Arc::new(std::sync::RwLock::new(state)),
		}
	}

	pub async fn id(&self, client: &dyn Client) -> Result<&Id> {
		self.store(client).await?;
		Ok(unsafe { &*(self.state.read().unwrap().id.as_ref().unwrap() as *const Id) })
	}

	#[async_recursion::async_recursion]
	pub async fn object(&self, client: &dyn Client) -> Result<&Object> {
		self.load(client).await?;
		Ok(unsafe { &*(self.state.read().unwrap().object.as_ref().unwrap() as *const Object) })
	}

	pub async fn try_get_object(&self, client: &dyn Client) -> Result<Option<&Object>> {
		if !self.try_load(client).await? {
			return Ok(None);
		}
		Ok(Some(unsafe {
			&*(self.state.read().unwrap().object.as_ref().unwrap() as *const Object)
		}))
	}

	pub async fn load(&self, client: &dyn Client) -> Result<()> {
		self.try_load(client)
			.await?
			.then_some(())
			.wrap_err("Failed to load the object.")
	}

	pub async fn try_load(&self, client: &dyn Client) -> Result<bool> {
		if self.state.read().unwrap().object.is_some() {
			return Ok(true);
		}
		let id = self.state.read().unwrap().id.clone().unwrap();
		let Some(bytes) = client.try_get_object(&id.clone().into()).await? else {
			return Ok(false);
		};
		let data = Data::deserialize(&bytes).wrap_err("Failed to deserialize the data.")?;
		let object = data.try_into()?;
		self.state.write().unwrap().object.replace(object);
		Ok(true)
	}

	pub async fn store(&self, client: &dyn Client) -> Result<()> {
		if self.state.read().unwrap().id.is_some() {
			return Ok(());
		}
		let data = self.data(client).await?;
		let bytes = data.serialize()?;
		let id = Id::new(&bytes);
		client
			.try_put_object(&id.clone().into(), &bytes)
			.await
			.wrap_err("Failed to put the object.")?
			.ok()
			.wrap_err("Expected all children to be stored.")?;
		self.state.write().unwrap().id.replace(id);
		Ok(())
	}

	#[async_recursion::async_recursion]
	pub async fn data(&self, client: &dyn Client) -> Result<Data> {
		let object = self.object(client).await?;
		let artifact = object.artifact.id(client).await?;
		let dependencies = object
			.dependencies
			.iter()
			.map(|(dependency, package)| async move {
				Ok::<_, Error>((dependency.clone(), package.id(client).await?.clone()))
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;
		Ok(Data {
			artifact,
			dependencies,
		})
	}
}

impl Package {
	pub async fn artifact(&self, client: &dyn Client) -> Result<&Artifact> {
		Ok(&self.object(client).await?.artifact)
	}

	pub async fn dependencies(&self, client: &dyn Client) -> Result<&BTreeMap<Dependency, Self>> {
		Ok(&self.object(client).await?.dependencies)
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

impl TryFrom<Data> for Object {
	type Error = Error;

	fn try_from(data: Data) -> std::result::Result<Self, Self::Error> {
		let artifact = Artifact::with_id(data.artifact);
		let dependencies = data
			.dependencies
			.into_iter()
			.map(|(dependency, id)| (dependency, Package::with_id(id)))
			.collect();
		Ok(Self {
			artifact,
			dependencies,
		})
	}
}

impl std::fmt::Display for Package {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.state.read().unwrap().id().as_ref().unwrap())?;
		Ok(())
	}
}

impl From<Id> for crate::Id {
	fn from(value: Id) -> Self {
		value.0
	}
}

impl TryFrom<crate::Id> for Id {
	type Error = Error;

	fn try_from(value: crate::Id) -> Result<Self, Self::Error> {
		if value.kind() != id::Kind::Package {
			return_error!("Invalid kind.");
		}
		Ok(Self(value))
	}
}

impl std::str::FromStr for Id {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		crate::Id::from_str(s)?.try_into()
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
