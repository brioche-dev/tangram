use crate::{
	checksum::Checksum, id, object, package, system::System, template, value, Build, Client, Error,
	Package, Result, Template, Value, WrapErr,
};
use bytes::Bytes;
use derive_more::Display;
use futures::{
	stream::{FuturesOrdered, FuturesUnordered},
	TryStreamExt,
};
use itertools::Itertools;
use std::{collections::BTreeMap, sync::Arc};
use tangram_error::return_error;

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
pub struct Target {
	state: Arc<std::sync::RwLock<State>>,
}

type State = object::State<Id, Object>;

/// A target object.
#[derive(Clone, Debug)]
pub struct Object {
	/// The system to build the target on.
	pub host: System,

	/// The target's executable.
	pub executable: Template,

	/// The target's package.
	pub package: Option<Package>,

	/// The target's name.
	pub name: Option<String>,

	/// The target's env.
	pub env: BTreeMap<String, Value>,

	/// The target's args.
	pub args: Vec<Value>,

	/// If a checksum of the target's output is provided, then the target will have access to the network.
	pub checksum: Option<Checksum>,

	/// If the target is marked as unsafe, then it will have access to the network even if a checksum is not provided.
	pub unsafe_: bool,
}

/// Target data.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Data {
	pub host: System,
	pub executable: template::Data,
	pub package: Option<package::Id>,
	pub name: Option<String>,
	pub env: BTreeMap<String, value::Data>,
	pub args: Vec<value::Data>,
	pub checksum: Option<Checksum>,
	pub unsafe_: bool,
}

impl Id {
	pub fn new(bytes: &Bytes) -> Self {
		Self(crate::Id::new_hashed(id::Kind::Target, bytes))
	}

	#[must_use]
	pub fn to_bytes(&self) -> Bytes {
		self.0.to_bytes()
	}
}

impl Target {
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

	pub async fn data(&self, client: &dyn Client) -> Result<Data> {
		let object = self.object(client).await?;
		Ok(Data {
			host: object.host.clone(),
			executable: object.executable.data(client).await?,
			package: if let Some(package) = &object.package {
				Some(package.id(client).await?.clone())
			} else {
				None
			},
			name: object.name.clone(),
			env: object
				.env
				.iter()
				.map(|(key, value)| async move {
					Ok::<_, Error>((key.clone(), value.data(client).await?))
				})
				.collect::<FuturesUnordered<_>>()
				.try_collect()
				.await?,
			args: object
				.args
				.iter()
				.map(|value| value.data(client))
				.collect::<FuturesOrdered<_>>()
				.try_collect()
				.await?,
			checksum: object.checksum.clone(),
			unsafe_: object.unsafe_,
		})
	}
}

impl Target {
	pub async fn host(&self, client: &dyn Client) -> Result<&System> {
		Ok(&self.object(client).await?.host)
	}

	pub async fn executable(&self, client: &dyn Client) -> Result<&Template> {
		Ok(&self.object(client).await?.executable)
	}

	pub async fn package(&self, client: &dyn Client) -> Result<&Option<Package>> {
		Ok(&self.object(client).await?.package)
	}

	pub async fn name(&self, client: &dyn Client) -> Result<&Option<String>> {
		Ok(&self.object(client).await?.name)
	}

	pub async fn env(&self, client: &dyn Client) -> Result<&BTreeMap<String, Value>> {
		Ok(&self.object(client).await?.env)
	}

	pub async fn args(&self, client: &dyn Client) -> Result<&Vec<Value>> {
		Ok(&self.object(client).await?.args)
	}

	pub async fn checksum(&self, client: &dyn Client) -> Result<&Option<Checksum>> {
		Ok(&self.object(client).await?.checksum)
	}

	pub async fn unsafe_(&self, client: &dyn Client) -> Result<bool> {
		Ok(self.object(client).await?.unsafe_)
	}

	pub async fn build(&self, client: &dyn Client) -> Result<Build> {
		let target_id = self.id(client).await?;
		let build_id = client.get_or_create_build_for_target(target_id).await?;
		let build = Build::with_id(build_id);
		Ok(build)
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
		std::iter::empty()
			.chain(self.executable.children())
			.chain(self.package.clone().map(Into::into))
			.chain(self.env.values().flat_map(value::Data::children))
			.chain(self.args.iter().flat_map(value::Data::children))
			.collect()
	}
}

impl TryFrom<Data> for Object {
	type Error = Error;

	fn try_from(data: Data) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			host: data.host,
			executable: data.executable.try_into()?,
			package: data.package.map(Package::with_id),
			name: data.name,
			env: data
				.env
				.into_iter()
				.map(|(key, data)| Ok::<_, Error>((key, data.try_into()?)))
				.try_collect()?,
			args: data.args.into_iter().map(TryInto::try_into).try_collect()?,
			checksum: data.checksum,
			unsafe_: data.unsafe_,
		})
	}
}

impl std::fmt::Display for Target {
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
		if value.kind() != id::Kind::Target {
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

#[derive(Clone, Debug)]
pub struct Builder {
	host: System,
	executable: Template,
	package: Option<Package>,
	name: Option<String>,
	env: BTreeMap<String, Value>,
	args: Vec<Value>,
	checksum: Option<Checksum>,
	unsafe_: bool,
}

impl Builder {
	#[must_use]
	pub fn new(host: System, executable: Template) -> Self {
		Self {
			host,
			executable,
			package: None,
			name: None,
			env: BTreeMap::new(),
			args: Vec::new(),
			checksum: None,
			unsafe_: false,
		}
	}

	#[must_use]
	pub fn host(mut self, host: System) -> Self {
		self.host = host;
		self
	}

	#[must_use]
	pub fn executable(mut self, executable: Template) -> Self {
		self.executable = executable;
		self
	}

	#[must_use]
	pub fn package(mut self, package: Package) -> Self {
		self.package = Some(package);
		self
	}

	#[must_use]
	pub fn name(mut self, name: String) -> Self {
		self.name = Some(name);
		self
	}

	#[must_use]
	pub fn env(mut self, env: BTreeMap<String, Value>) -> Self {
		self.env = env;
		self
	}

	#[must_use]
	pub fn args(mut self, args: Vec<Value>) -> Self {
		self.args = args;
		self
	}

	#[must_use]
	pub fn checksum(mut self, checksum: Option<Checksum>) -> Self {
		self.checksum = checksum;
		self
	}

	#[must_use]
	pub fn unsafe_(mut self, unsafe_: bool) -> Self {
		self.unsafe_ = unsafe_;
		self
	}

	#[must_use]
	pub fn build(self) -> Target {
		Target::with_object(Object {
			package: self.package,
			host: self.host,
			executable: self.executable,
			name: self.name,
			env: self.env,
			args: self.args,
			checksum: self.checksum,
			unsafe_: self.unsafe_,
		})
	}
}
