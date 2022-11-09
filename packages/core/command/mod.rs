use std::{
	collections::{BTreeMap, HashMap},
	path::PathBuf,
};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

// The fully resolved command for a process with arguments and environment variables, plus all paths required to run the process in the sandbox.
pub struct Command {
	#[cfg(target_os = "linux")]
	pub chroot_path: PathBuf,
	#[cfg(target_os = "linux")]
	pub has_base: bool,
	pub current_dir: PathBuf,
	pub envs: BTreeMap<String, String>,
	pub command: PathBuf,
	pub args: Vec<String>,
	pub paths: HashMap<PathBuf, PathMode>,
	pub enable_network_access: bool,
}

// The mode used to mount a path in the sandbox. Ordered from least permissive to most permissive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PathMode {
	Read,
	ReadWrite,
	ReadWriteCreate,
}
