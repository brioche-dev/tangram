pub use self::{builder::Builder, data::Data, error::Error, server::Server};
use crate::{checksum::Checksum, operation, system::System, template::Template};
use std::collections::BTreeMap;

mod builder;
mod data;
mod error;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod new;
mod run;
mod server;

/// A process.
#[allow(clippy::unsafe_derive_deserialize)]
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Process {
	/// The hash.
	hash: operation::Hash,

	/// The system the process will run on.
	system: System,

	/// The executable to run.
	executable: Template,

	/// The environment variables to set.
	#[serde(default)]
	env: BTreeMap<String, Template>,

	/// The arguments to pass to the command.
	#[serde(default)]
	args: Vec<Template>,

	/// A checksum of the process's output. If a checksum is provided, then unsafe options can be used.
	#[serde(default)]
	checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[serde(default, rename = "unsafe")]
	is_unsafe: bool,

	/// If this flag is set, then the process will have access to the network. This is an unsafe option.
	#[serde(default)]
	network: bool,

	/// A set of paths on the host's file system to expose to the process. This is an unsafe option.
	#[serde(default)]
	host_paths: Vec<String>,
}

impl Process {
	/// Get the hash.
	#[must_use]
	pub fn hash(&self) -> operation::Hash {
		self.hash
	}

	/// Get the system the process will run on.
	#[must_use]
	pub fn system(&self) -> System {
		self.system
	}

	/// Get the executable to run.
	#[must_use]
	pub fn executable(&self) -> &Template {
		&self.executable
	}

	/// Get the environment variables to set.
	#[must_use]
	pub fn env(&self) -> &BTreeMap<String, Template> {
		&self.env
	}

	/// Get the arguments to pass to the command.
	#[must_use]
	pub fn args(&self) -> &[Template] {
		&self.args
	}

	/// Get a checksum of the process's output. If a checksum is provided, then unsafe options can be used.
	#[must_use]
	pub fn checksum(&self) -> &Option<Checksum> {
		&self.checksum
	}

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[must_use]
	pub fn is_unsafe(&self) -> bool {
		self.is_unsafe
	}

	/// If this flag is set, then the process will have access to the network. This is an unsafe option.
	#[must_use]
	pub fn network(&self) -> bool {
		self.network
	}

	/// Get a set of paths on the host's file system to expose to the process. This is an unsafe option.
	#[must_use]
	pub fn host_paths(&self) -> &[String] {
		&self.host_paths
	}
}
