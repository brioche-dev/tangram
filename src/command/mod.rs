pub use self::{builder::Builder, data::Data, error::Error};
use crate::{checksum::Checksum, operation, system::System, template::Template};
use std::collections::BTreeMap;

mod builder;
mod data;
mod error;
#[cfg(all(target_os = "linux", feature = "operation_run"))]
mod linux;
#[cfg(all(target_os = "macos", feature = "operation_run"))]
mod macos;
mod new;
#[cfg(feature = "operation_run")]
mod run;
#[cfg(feature = "operation_run")]
mod sandbox_disabled;

/// A process.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
	/// The hash.
	hash: operation::Hash,

	/// The system the command will run on.
	system: System,

	/// The executable to run.
	executable: Template,

	/// The environment variables to set.
	#[serde(default)]
	env: BTreeMap<String, Template>,

	/// The arguments to pass to the command.
	#[serde(default)]
	args: Vec<Template>,

	/// A checksum of the command's output. If a checksum is provided, then unsafe options can be used.
	#[serde(default)]
	checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[serde(default, rename = "unsafe")]
	unsafe_: bool,

	/// If this flag is set, then the process will have access to the network. This is an unsafe option.
	#[serde(default)]
	network: bool,

	/// A set of paths on the host's file system to expose to the process. This is an unsafe option.
	#[serde(default)]
	host_paths: Vec<String>,
}

impl Command {
	#[must_use]
	pub fn hash(&self) -> operation::Hash {
		self.hash
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

	#[must_use]
	pub fn host_paths(&self) -> &[String] {
		&self.host_paths
	}
}
