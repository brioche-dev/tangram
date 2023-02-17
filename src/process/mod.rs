use crate::{checksum::Checksum, system::System, template::Template};
use std::collections::BTreeMap;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod run;

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
	#[buffalo(id = 0)]
	pub system: System,

	#[buffalo(id = 1)]
	pub env: BTreeMap<String, Template>,

	#[buffalo(id = 2)]
	pub command: Template,

	#[buffalo(id = 3)]
	pub args: Vec<Template>,

	#[buffalo(id = 4)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 5)]
	#[serde(rename = "unsafe")]
	pub is_unsafe: bool,
}
