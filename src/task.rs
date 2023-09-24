use crate::{
	checksum::Checksum, id, object, package, system::System, template, value, Client, Package,
	Result, Run, Template, Value,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Task(Handle);

crate::object!(Task);

/// A task object.
#[derive(Clone, Debug)]
pub(crate) struct Object {
	/// The task's package.
	pub package: Option<Package>,

	/// The system to run the task on.
	pub host: System,

	/// The task's executable.
	pub executable: Template,

	/// The task's target.
	pub target: Option<String>,

	/// The task's environment variables.
	pub env: BTreeMap<String, Value>,

	/// The task's command line arguments.
	pub args: Vec<Value>,

	/// A checksum of the task's output. If a checksum is provided, then unsafe options can be used.
	pub checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	pub unsafe_: bool,

	/// If this flag is set, then the task will have access to the network. This is an unsafe option.
	pub network: bool,
}

/// A task.
#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub(crate) struct Data {
	/// The target's package.
	#[tangram_serialize(id = 0)]
	pub package: Option<package::Id>,

	/// The system to run the task on.
	#[tangram_serialize(id = 1)]
	pub host: System,

	/// The task's executable.
	#[tangram_serialize(id = 3)]
	pub executable: template::Data,

	/// The task's target.
	#[tangram_serialize(id = 2)]
	pub target: Option<String>,

	/// The task's environment variables.
	#[tangram_serialize(id = 4)]
	pub env: BTreeMap<String, value::Data>,

	/// The task's command line arguments.
	#[tangram_serialize(id = 5)]
	pub args: Vec<value::Data>,

	/// A checksum of the task's output. If a checksum is provided, then unsafe options can be used.
	#[tangram_serialize(id = 6)]
	pub checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[tangram_serialize(id = 7)]
	pub unsafe_: bool,

	/// If this flag is set, then the task will have access to the network. This is an unsafe option.
	#[tangram_serialize(id = 8)]
	pub network: bool,
}

impl Task {
	#[must_use]
	pub fn handle(&self) -> &Handle {
		&self.0
	}

	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(Handle::with_id(id))
	}

	pub async fn run(&self, client: &Client) -> Result<Run> {
		Ok(Run::with_id(
			client.run(self.handle().id(client).await?).await?,
		))
	}
}

impl Id {
	#[must_use]
	pub fn with_data_bytes(bytes: &[u8]) -> Self {
		Self(crate::Id::new_hashed(id::Kind::Task, bytes))
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		Data {
			package: self
				.package
				.as_ref()
				.map(|package| package.handle().expect_id()),
			host: self.host,
			target: self.target.clone(),
			executable: self.executable.to_data(),
			env: self
				.env
				.iter()
				.map(|(key, value)| (key.clone(), value.to_data()))
				.collect(),
			args: self.args.iter().map(Value::to_data).collect(),
			checksum: self.checksum.clone(),
			unsafe_: self.unsafe_,
			network: self.network,
		}
	}

	#[must_use]
	pub(crate) fn from_data(data: Data) -> Self {
		Self {
			package: data.package.map(Package::with_id),
			host: data.host,
			target: data.target,
			executable: Template::from_data(data.executable),
			env: data
				.env
				.into_iter()
				.map(|(key, data)| (key, Value::from_data(data)))
				.collect(),
			args: data.args.into_iter().map(Value::from_data).collect(),
			checksum: data.checksum,
			unsafe_: data.unsafe_,
			network: data.network,
		}
	}

	pub fn children(&self) -> Vec<object::Handle> {
		std::iter::empty()
			.chain(self.executable.children())
			.chain(self.env.values().flat_map(value::Value::children))
			.chain(self.args.iter().flat_map(value::Value::children))
			.collect()
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<object::Id> {
		std::iter::empty()
			.chain(self.executable.children())
			.chain(self.env.values().flat_map(value::Data::children))
			.chain(self.args.iter().flat_map(value::Data::children))
			.collect()
	}
}

#[derive(Clone, Debug)]
pub struct Builder {
	package: Option<Package>,
	host: System,
	executable: Template,
	target: Option<String>,
	env: BTreeMap<String, Value>,
	args: Vec<Value>,
	checksum: Option<Checksum>,
	unsafe_: bool,
	network: bool,
}

impl Builder {
	#[must_use]
	pub fn new(host: System, executable: Template) -> Self {
		Self {
			package: None,
			host,
			executable,
			target: None,
			env: BTreeMap::new(),
			args: Vec::new(),
			checksum: None,
			unsafe_: false,
			network: false,
		}
	}

	#[must_use]
	pub fn package(mut self, package: Package) -> Self {
		self.package = Some(package);
		self
	}

	#[must_use]
	pub fn system(mut self, host: System) -> Self {
		self.host = host;
		self
	}

	#[must_use]
	pub fn executable(mut self, executable: Template) -> Self {
		self.executable = executable;
		self
	}

	#[must_use]
	pub fn target(mut self, target: String) -> Self {
		self.target = Some(target);
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
	pub fn network(mut self, network: bool) -> Self {
		self.network = network;
		self
	}

	#[must_use]
	pub fn build(self) -> Task {
		Task(Handle::with_object(Object {
			package: self.package,
			host: self.host,
			executable: self.executable,
			target: self.target,
			env: self.env,
			args: self.args,
			checksum: self.checksum,
			unsafe_: self.unsafe_,
			network: self.network,
		}))
	}
}
