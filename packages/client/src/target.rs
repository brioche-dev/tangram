use crate::{
	checksum::Checksum, object, package, system::System, template, value, Build, Client, Package,
	Result, Template, Value, WrapErr,
};
use bytes::Bytes;
use std::collections::BTreeMap;

crate::id!(Target);
crate::handle!(Target);

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Id(crate::Id);

#[derive(Clone, Debug)]
pub struct Target(object::Handle);

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

impl Object {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			host: self.host.clone(),
			executable: self.executable.to_data(),
			package: self.package.as_ref().map(Package::expect_id).cloned(),
			name: self.name.clone(),
			env: self
				.env
				.iter()
				.map(|(key, value)| (key.clone(), value.clone().into()))
				.collect(),
			args: self.args.iter().cloned().map(Into::into).collect(),
			checksum: self.checksum.clone(),
			unsafe_: self.unsafe_,
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self {
			host: data.host,
			executable: Template::from_data(data.executable),
			package: data.package.map(Package::with_id),
			name: data.name,
			env: data
				.env
				.into_iter()
				.map(|(key, data)| (key, data.into()))
				.collect(),
			args: data.args.into_iter().map(Into::into).collect(),
			checksum: data.checksum,
			unsafe_: data.unsafe_,
		}
	}

	pub fn children(&self) -> Vec<object::Handle> {
		std::iter::empty()
			.chain(self.executable.children())
			.chain(self.package.iter().map(|package| package.handle().clone()))
			.chain(self.env.values().flat_map(value::Value::children))
			.chain(self.args.iter().flat_map(value::Value::children))
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
		std::iter::empty()
			.chain(self.executable.children())
			.chain(self.package.clone().map(Into::into))
			.chain(self.env.values().flat_map(value::Data::children))
			.chain(self.args.iter().flat_map(value::Data::children))
			.collect()
	}
}

impl std::fmt::Display for Target {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.expect_id())?;
		Ok(())
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
