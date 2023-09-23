use crate::{
	checksum::Checksum, object, package, system::System, template, value, Client, Package, Result,
	Run, Template, Value,
};
use std::collections::BTreeMap;
use thiserror::Error;

crate::object!(Task);

/// A task object.
#[derive(Clone, Debug)]
pub(crate) struct Object {
	/// The tasks's package.
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

impl Handle {
	pub async fn run(&self, client: &Client) -> Result<Run> {
		todo!()
	}
}

impl Object {
	#[must_use]
	pub(crate) fn to_data(&self) -> Data {
		Data {
			package: self.package.as_ref().map(Package::expect_id),
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
	pub fn build(self) -> Handle {
		Handle::with_object(Object {
			package: self.package,
			host: self.host,
			executable: self.executable,
			target: self.target,
			env: self.env,
			args: self.args,
			checksum: self.checksum,
			unsafe_: self.unsafe_,
			network: self.network,
		})
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

// /// An error from a target.
// #[derive(
// 	Clone,
// 	Debug,
// 	Error,
// 	serde::Serialize,
// 	serde::Deserialize,
// 	tangram_serialize::Deserialize,
// 	tangram_serialize::Serialize,
// )]
// #[serde(rename_all = "camelCase")]
// pub struct Error {
// 	#[tangram_serialize(id = 0)]
// 	pub message: String,
// 	#[tangram_serialize(id = 1)]
// 	pub location: Option<Location>,
// 	#[tangram_serialize(id = 2)]
// 	pub stack_trace: Option<StackTrace>,
// 	#[tangram_serialize(id = 3)]
// 	pub source: Option<Arc<crate::Error>>,
// }

// /// A stack trace.
// #[derive(
// 	Clone,
// 	Debug,
// 	serde::Serialize,
// 	serde::Deserialize,
// 	tangram_serialize::Deserialize,
// 	tangram_serialize::Serialize,
// )]
// #[serde(rename_all = "camelCase")]
// pub struct StackTrace {
// 	#[tangram_serialize(id = 0)]
// 	pub stack_frames: Vec<StackFrame>,
// }

// /// A stack frame.
// #[derive(
// 	Clone,
// 	Debug,
// 	serde::Serialize,
// 	serde::Deserialize,
// 	tangram_serialize::Deserialize,
// 	tangram_serialize::Serialize,
// )]
// #[serde(rename_all = "camelCase")]
// pub struct StackFrame {
// 	#[tangram_serialize(id = 0)]
// 	pub location: Option<Location>,
// }

// /// A source location.
// #[derive(
// 	Clone,
// 	Debug,
// 	serde::Serialize,
// 	serde::Deserialize,
// 	tangram_serialize::Deserialize,
// 	tangram_serialize::Serialize,
// )]
// #[serde(rename_all = "camelCase")]
// pub struct Location {
// 	#[tangram_serialize(id = 0)]
// 	pub source: Source,
// 	#[tangram_serialize(id = 1)]
// 	pub position: Position,
// }

// /// A source.
// #[derive(
// 	Clone,
// 	Debug,
// 	serde::Serialize,
// 	serde::Deserialize,
// 	tangram_serialize::Deserialize,
// 	tangram_serialize::Serialize,
// )]
// #[serde(rename_all = "snake_case", tag = "kind", content = "value")]
// pub enum Source {
// 	#[tangram_serialize(id = 0)]
// 	Global(Option<String>),
// 	#[tangram_serialize(id = 1)]
// 	Module(Url),
// }

// impl std::fmt::Display for Error {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		// Write the message.
// 		write!(f, "{}", self.message)?;

// 		// Write the stack trace.
// 		if let Some(stack_trace) = &self.stack_trace {
// 			write!(f, "{stack_trace}")?;
// 		}

// 		Ok(())
// 	}
// }

// impl std::fmt::Display for StackTrace {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		for stack_frame in &self.stack_frames {
// 			writeln!(f)?;
// 			write!(f, "  {stack_frame}")?;
// 		}
// 		Ok(())
// 	}
// }

// impl std::fmt::Display for StackFrame {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		if let Some(location) = &self.location {
// 			write!(f, "{location}")?;
// 		} else {
// 			write!(f, "[unknown]")?;
// 		}
// 		Ok(())
// 	}
// }

// impl std::fmt::Display for Location {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		let source = &self.source;
// 		let line = self.position.line + 1;
// 		let character = self.position.character + 1;
// 		write!(f, "{source}:{line}:{character}")?;
// 		Ok(())
// 	}
// }

// impl std::fmt::Display for Source {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		match self {
// 			Source::Global(path) => {
// 				let path = path.as_deref().unwrap_or("[unknown]");
// 				write!(f, "global:{path}")?;
// 			},

// 			Source::Module(module) => {
// 				write!(f, "{module}")?;
// 			},
// 		}
// 		Ok(())
// 	}
// }
