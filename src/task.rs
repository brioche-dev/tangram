use crate::{checksum::Checksum, system::System, template};
use std::collections::BTreeMap;
use thiserror::Error;

crate::id!(Task);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(Task);

/// A task.
#[derive(Clone, Debug)]
pub struct Value {
	/// The system to run the task on.
	pub host: System,

	/// The task's executable.
	pub executable: template::Value,

	/// The task's environment variables.
	pub env: BTreeMap<String, template::Value>,

	/// The task's command line arguments.
	pub args: Vec<template::Value>,

	/// A checksum of the task's output. If a checksum is provided, then unsafe options can be used.
	pub checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	pub unsafe_: bool,

	/// If this flag is set, then the task will have access to the network. This is an unsafe option.
	pub network: bool,
}

crate::value!(Task);

/// A task.
#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	/// The system to run the task on.
	#[tangram_serialize(id = 0)]
	pub host: System,

	/// The task's executable.
	#[tangram_serialize(id = 1)]
	pub executable: template::Data,

	/// The task's environment variables.
	#[tangram_serialize(id = 2)]
	pub env: BTreeMap<String, template::Data>,

	/// The task's command line arguments.
	#[tangram_serialize(id = 3)]
	pub args: Vec<template::Data>,

	/// A checksum of the task's output. If a checksum is provided, then unsafe options can be used.
	#[tangram_serialize(id = 4)]
	pub checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[tangram_serialize(id = 5)]
	pub unsafe_: bool,

	/// If this flag is set, then the task will have access to the network. This is an unsafe option.
	#[tangram_serialize(id = 6)]
	pub network: bool,
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self {
			host: data.host,
			executable: template::Value::from_data(data.executable),
			env: data
				.env
				.into_iter()
				.map(|(key, data)| (key, template::Value::from_data(data)))
				.collect(),
			args: data
				.args
				.into_iter()
				.map(template::Value::from_data)
				.collect(),
			checksum: data.checksum,
			unsafe_: data.unsafe_,
			network: data.network,
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			host: self.host,
			executable: self.executable.to_data(),
			env: self
				.env
				.iter()
				.map(|(key, value)| (key.clone(), value.to_data()))
				.collect(),
			args: self.args.iter().map(template::Value::to_data).collect(),
			checksum: self.checksum.clone(),
			unsafe_: self.unsafe_,
			network: self.network,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		self.executable
			.children()
			.into_iter()
			.chain(self.env.values().flat_map(template::Value::children))
			.chain(self.args.iter().flat_map(template::Value::children))
			.collect()
	}

	#[must_use]
	pub fn new(
		host: System,
		executable: template::Value,
		env: BTreeMap<String, template::Value>,
		args: Vec<template::Value>,
		checksum: Option<Checksum>,
		unsafe_: bool,
		network: bool,
	) -> Self {
		Self {
			host,
			executable,
			env,
			args,
			checksum,
			unsafe_,
			network,
		}
	}

	#[must_use]
	pub fn host(&self) -> System {
		self.host
	}

	#[must_use]
	pub fn executable(&self) -> &template::Value {
		&self.executable
	}

	#[must_use]
	pub fn env(&self) -> &BTreeMap<String, template::Value> {
		&self.env
	}

	#[must_use]
	pub fn args(&self) -> &[template::Value] {
		&self.args
	}

	#[must_use]
	pub fn checksum(&self) -> &Option<Checksum> {
		&self.checksum
	}

	#[must_use]
	pub fn unsafe_(&self) -> bool {
		self.unsafe_
	}

	#[must_use]
	pub fn network(&self) -> bool {
		self.network
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		self.executable
			.children()
			.into_iter()
			.chain(self.env.values().flat_map(template::Data::children))
			.chain(self.args.iter().flat_map(template::Data::children))
			.collect()
	}
}

#[derive(Clone, Debug)]
pub struct Builder {
	host: System,
	executable: template::Value,
	env: BTreeMap<String, template::Value>,
	args: Vec<template::Value>,
	checksum: Option<Checksum>,
	unsafe_: bool,
	network: bool,
}

impl Builder {
	#[must_use]
	pub fn new(host: System, executable: template::Value) -> Self {
		Self {
			host,
			executable,
			env: BTreeMap::new(),
			args: Vec::new(),
			checksum: None,
			unsafe_: false,
			network: false,
		}
	}

	#[must_use]
	pub fn system(mut self, host: System) -> Self {
		self.host = host;
		self
	}

	#[must_use]
	pub fn executable(mut self, executable: template::Value) -> Self {
		self.executable = executable;
		self
	}

	#[must_use]
	pub fn env(mut self, env: BTreeMap<String, template::Value>) -> Self {
		self.env = env;
		self
	}

	#[must_use]
	pub fn args(mut self, args: Vec<template::Value>) -> Self {
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
	pub fn build(self) -> Handle {
		Handle::with_value(Value::new(
			self.host,
			self.executable,
			self.env,
			self.args,
			self.checksum,
			self.unsafe_,
			self.network,
		))
	}
}

/// An error from a task.
#[derive(
	Clone,
	Debug,
	Error,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Error {
	#[error(r#"The process exited with code {0}."#)]
	#[tangram_serialize(id = 0)]
	Code(i32),

	#[error(r#"The process exited with signal {0}."#)]
	#[tangram_serialize(id = 1)]
	Signal(i32),
}
