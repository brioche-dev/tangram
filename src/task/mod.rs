pub use self::error::Error;
use crate::{checksum::Checksum, system::System, template};
use std::collections::BTreeMap;

// mod basic;
// mod builder;
mod error;
// #[cfg(target_os = "linux))]
// mod linux;
// #[cfg(target_os = "macos")]
// mod macos;
// mod run;

crate::id!();

crate::kind!(Task);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

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

/// A task.
#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
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
		Value {
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
		todo!()
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
	pub fn children(&self) -> Vec<crate::Handle> {
		self.executable
			.children()
			.into_iter()
			.chain(self.env.values().flat_map(template::Value::children))
			.chain(self.args.iter().flat_map(template::Value::children))
			.collect()
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
