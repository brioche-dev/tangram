use crate::{id, object, return_error, Artifact, Client, Dependency, Error, Result, WrapErr};
use async_recursion::async_recursion;
use bytes::Bytes;
use derive_more::Display;
use futures::{stream::FuturesUnordered, TryStreamExt};
use itertools::Itertools;
use std::{collections::BTreeMap, sync::Arc};
use tangram_error::error;

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
pub struct Lock {
	state: Arc<std::sync::RwLock<State>>,
}

type State = object::State<Id, Object>;

#[derive(Clone, Debug)]
pub struct Object {
	pub dependencies: BTreeMap<Dependency, Entry>,
}

#[derive(Clone, Debug)]
pub struct Entry {
	pub package: Artifact,
	pub lock: Lock,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Data {
	pub dependencies: BTreeMap<Dependency, data::Entry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockFile {
	pub root: data::Entry,
	pub entries: BTreeMap<Id, BTreeMap<Dependency, data::Entry>>,
}

pub mod data {
	use crate::artifact;

	#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
	pub struct Entry {
		pub package: artifact::Id,
		pub lock: super::Id,
	}
}

impl Id {
	pub fn new(bytes: &Bytes) -> Self {
		Self(crate::Id::new_hashed(id::Kind::Lock, bytes))
	}

	#[must_use]
	pub fn to_bytes(&self) -> Bytes {
		self.0.to_bytes()
	}
}

impl Lock {
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
		let dependencies = object
			.dependencies
			.iter()
			.map(|(dependency, entry)| async move {
				Ok::<_, Error>((dependency.clone(), entry.data(client).await?))
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;
		Ok(Data { dependencies })
	}
}

impl Lock {
	pub async fn dependencies(&self, client: &dyn Client) -> Result<&BTreeMap<Dependency, Entry>> {
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
		self.dependencies
			.values()
			.flat_map(|entry| [entry.package.clone().into(), entry.lock.clone().into()])
			.collect()
	}
}

impl Entry {
	pub async fn data(&self, client: &dyn Client) -> Result<data::Entry> {
		Ok(data::Entry {
			package: self.package.id(client).await?.clone(),
			lock: self.lock.id(client).await?.clone(),
		})
	}
}

impl LockFile {
	pub async fn with_package_and_lock(
		client: &dyn Client,
		package: Artifact,
		lock: Lock,
	) -> tangram_error::Result<Self> {
		let mut entries = BTreeMap::new();
		Self::with_lock_inner(client, lock.clone(), &mut entries).await?;
		let package = package.id(client).await?;
		let lock = lock.id(client).await?.clone();
		let root = data::Entry { package, lock };
		Ok(Self { root, entries })
	}

	#[async_recursion]
	async fn with_lock_inner(
		client: &dyn Client,
		lock: Lock,
		entries: &mut BTreeMap<Id, BTreeMap<Dependency, data::Entry>>,
	) -> tangram_error::Result<()> {
		// Get the ID and check if we've already visited this lock.
		let id = lock.id(client).await.wrap_err("Failed to get ID")?.clone();
		if entries.contains_key(&id) {
			return Ok(());
		}

		// Add the data to the lockfile.
		let data = lock.data(client).await.wrap_err("Failed to get data.")?;
		entries.insert(id, data.dependencies.clone());

		// Visit any dependencies.
		for entry in lock.object(client).await?.dependencies.values() {
			Self::with_lock_inner(client, entry.lock.clone(), entries).await?;
		}
		Ok(())
	}

	pub fn to_package_and_lock(&self) -> tangram_error::Result<(Artifact, Lock)> {
		let id = &self.root.lock;
		let package = Artifact::with_id(self.root.package.clone());
		let Entry { package, lock } = self.to_lock_inner(id, package)?;
		Ok((package, lock))
	}

	fn to_lock_inner(&self, id: &Id, package: Artifact) -> tangram_error::Result<Entry> {
		let raw_dependencies = self
			.entries
			.get(id)
			.ok_or(error!("Lockfile is corrupted."))?;

		let mut dependencies = BTreeMap::new();
		for (dependency, entry) in raw_dependencies {
			let package = Artifact::with_id(entry.package.clone());
			let entry = self.to_lock_inner(&entry.lock, package)?;
			let _ = dependencies.insert(dependency.clone(), entry);
		}

		let object = Object { dependencies };

		let lock = Lock::with_object(object);
		let entry = Entry { package, lock };
		Ok(entry)
	}
}

impl TryFrom<Data> for Object {
	type Error = Error;

	fn try_from(data: Data) -> std::result::Result<Self, Self::Error> {
		let dependencies = data
			.dependencies
			.into_iter()
			.map(|(dependency, entry)| Ok::<_, Error>((dependency, entry.try_into()?)))
			.try_collect()?;
		Ok(Self { dependencies })
	}
}

impl TryFrom<data::Entry> for Entry {
	type Error = Error;

	fn try_from(value: data::Entry) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			package: Artifact::with_id(value.package),
			lock: Lock::with_id(value.lock),
		})
	}
}

impl std::fmt::Display for Lock {
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
		if value.kind() != id::Kind::Lock {
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
