use crate::{
	checksum::Checksum, object, package, return_error, system::System, template, value, Build,
	Client, Package, Result, Template, Value,
};
use std::collections::BTreeMap;

crate::id!(Task);
crate::handle!(Target);

#[derive(Clone, Copy, Debug)]
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

	/// The target's environment variables.
	pub env: BTreeMap<String, Value>,

	/// The target's command line arguments.
	pub args: Vec<Value>,

	/// If a checksum of the target's output is provided, then the target will have access to the network.
	pub checksum: Option<Checksum>,

	/// If the target is marked as unsafe, then it will have access to the network even if a checksum is not provided.
	pub unsafe_: bool,
}

/// Target data.
#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	/// The system to build the target on.
	#[tangram_serialize(id = 0)]
	pub host: System,

	/// The task's executable.
	#[tangram_serialize(id = 1)]
	pub executable: template::Data,

	/// The target's package.
	#[tangram_serialize(id = 2)]
	pub package: Option<package::Id>,

	/// The task's target.
	#[tangram_serialize(id = 3)]
	pub name: Option<String>,

	/// The task's environment variables.
	#[tangram_serialize(id = 4)]
	pub env: BTreeMap<String, value::Data>,

	/// The task's command line arguments.
	#[tangram_serialize(id = 5)]
	pub args: Vec<value::Data>,

	/// If a checksum of the task's output is provided, then the target will have access to the network.
	#[tangram_serialize(id = 6)]
	pub checksum: Option<Checksum>,

	/// If the target is marked as unsafe, then it will have access to the network even if a checksum is not provided.
	#[tangram_serialize(id = 7)]
	pub unsafe_: bool,
}

impl Target {
	pub async fn host(&self, client: &Client) -> Result<&System> {
		Ok(&self.object(client).await?.host)
	}

	pub async fn executable(&self, client: &Client) -> Result<&Template> {
		Ok(&self.object(client).await?.executable)
	}

	pub async fn package(&self, client: &Client) -> Result<&Option<Package>> {
		Ok(&self.object(client).await?.package)
	}

	pub async fn build(&self, client: &Client) -> Result<Build> {
		let target_id = self.id(client).await?;
		let build_id = client.get_or_create_build_for_target(target_id).await?;
		let build = Build::with_id(build_id);
		Ok(build)
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		Data {
			host: self.host.clone(),
			executable: self.executable.to_data(),
			package: self.package.as_ref().map(Package::expect_id),
			name: self.name.clone(),
			env: self
				.env
				.iter()
				.map(|(key, value)| (key.clone(), value.to_data()))
				.collect(),
			args: self.args.iter().map(Value::to_data).collect(),
			checksum: self.checksum.clone(),
			unsafe_: self.unsafe_,
		}
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		Self {
			host: data.host,
			executable: Template::from_data(data.executable),
			package: data.package.map(Package::with_id),
			name: data.name,
			env: data
				.env
				.into_iter()
				.map(|(key, data)| (key, Value::from_data(data)))
				.collect(),
			args: data.args.into_iter().map(Value::from_data).collect(),
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
	pub(crate) fn serialize(&self) -> Result<Vec<u8>> {
		let mut bytes = Vec::new();
		byteorder::WriteBytesExt::write_u8(&mut bytes, 0)?;
		tangram_serialize::to_writer(self, &mut bytes)?;
		Ok(bytes)
	}

	pub(crate) fn deserialize(mut bytes: &[u8]) -> Result<Self> {
		let version = byteorder::ReadBytesExt::read_u8(&mut bytes)?;
		if version != 0 {
			return_error!(r#"Cannot deserialize this object with version "{version}"."#);
		}
		let value = tangram_serialize::from_reader(bytes)?;
		Ok(value)
	}

	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		std::iter::empty()
			.chain(self.executable.children())
			.chain(self.package.map(Into::into))
			.chain(self.env.values().flat_map(value::Data::children))
			.chain(self.args.iter().flat_map(value::Data::children))
			.collect()
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
