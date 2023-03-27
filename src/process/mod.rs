pub use self::error::Error;
use crate::{checksum::Checksum, system::System, template::Template};
use std::collections::BTreeMap;

mod client;
mod error;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod run;
pub mod server;

/// A process.
#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Process {
	/// The system the process will run on.
	#[buffalo(id = 0)]
	pub system: System,

	/// The command to run.
	#[buffalo(id = 1)]
	pub command: Template,

	/// The environment variables to set.
	#[buffalo(id = 2)]
	pub env: BTreeMap<String, Template>,

	/// The arguments to pass to the command.
	#[buffalo(id = 3)]
	pub args: Vec<Template>,

	/// A checksum of the process's output. If a checksum is provided, then unsafe options can be used.
	#[buffalo(id = 4)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	/// If this flag is set, then unsafe options can be used without a checksum.
	#[buffalo(id = 5)]
	#[serde(rename = "unsafe")]
	pub is_unsafe: bool,

	/// If this flag is set, then the process will have access to the network. This is an unsafe option.
	#[buffalo(id = 6)]
	#[serde(default)]
	pub network: bool,

	/// If this flag is set, then the process will have access to the specified paths on the host's file system. This is an unsafe option.
	#[buffalo(id = 7)]
	#[serde(default)]
	pub host_paths: Vec<String>,
}
