pub use self::{builder::Builder, data::Data, error::Error};
use crate::{block::Block, checksum::Checksum, system::System, template::Template};
use std::collections::BTreeMap;

#[cfg(feature = "evaluate")]
mod basic;
mod builder;
mod data;
mod error;
#[cfg(all(target_os = "linux", feature = "evaluate"))]
mod linux;
#[cfg(all(target_os = "macos", feature = "evaluate"))]
mod macos;
mod new;
#[cfg(feature = "evaluate")]
mod run;

/// A task.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
	/// The task's block.
	block: Block,

	/// The system to run the task on.
	system: System,

	/// The task's executable.
	executable: Template,

	/// The task's environment variables.
	#[serde(default)]
	env: BTreeMap<String, Template>,

	/// The task's command line arguments.
	#[serde(default)]
	args: Vec<Template>,

	/// A checksum of the task's output. If a checksum is provided, then unsafe options can be used.
	#[serde(default)]
	checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[serde(default, rename = "unsafe")]
	unsafe_: bool,

	/// If this flag is set, then the process will have access to the network. This is an unsafe option.
	#[serde(default)]
	network: bool,
}

impl Task {
	#[must_use]
	pub fn block(&self) -> Block {
		self.block
	}

	#[must_use]
	pub fn system(&self) -> System {
		self.system
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
