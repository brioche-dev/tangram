pub use self::{builder::Builder, error::Error};
use crate as tg;
use crate::{checksum::Checksum, system::System, template::Template};
use std::collections::BTreeMap;

#[cfg(feature = "build")]
mod basic;
mod builder;
mod error;
#[cfg(all(target_os = "linux", feature = "build"))]
mod linux;
#[cfg(all(target_os = "macos", feature = "build"))]
mod macos;
#[cfg(feature = "build")]
mod run;

/// A task.
#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Task {
	/// The system to run the task on.
	#[tangram_serialize(id = 0)]
	host: System,

	/// The task's executable.
	#[tangram_serialize(id = 1)]
	executable: Template,

	/// The task's environment variables.
	#[tangram_serialize(id = 2)]
	env: BTreeMap<String, Template>,

	/// The task's command line arguments.
	#[tangram_serialize(id = 3)]
	args: Vec<Template>,

	/// A checksum of the task's output. If a checksum is provided, then unsafe options can be used.
	#[tangram_serialize(id = 4)]
	checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[tangram_serialize(id = 5)]
	unsafe_: bool,

	/// If this flag is set, then the task will have access to the network. This is an unsafe option.
	#[tangram_serialize(id = 6)]
	network: bool,
}

crate::value!(Task);

impl Task {
	#[must_use]
	pub fn new(
		host: System,
		executable: Template,
		env: BTreeMap<String, Template>,
		args: Vec<Template>,
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
	pub fn children(&self) -> Vec<tg::Value> {
		vec![]
	}

	#[must_use]
	pub fn host(&self) -> System {
		self.host
	}

	#[must_use]
	pub fn executable(&self) -> &Template {
		&self.executable
	}

	#[must_use]
	pub fn env(&self) -> &BTreeMap<String, Template> {
		&self.env
	}

	#[must_use]
	pub fn args(&self) -> &[Template] {
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
