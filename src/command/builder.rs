use super::Command;
use crate::{
	checksum::Checksum, error::Result, instance::Instance, system::System, template::Template,
};
use std::collections::BTreeMap;

impl Command {
	#[must_use]
	pub fn builder(system: System, executable: Template) -> Builder {
		Builder::new(system, executable)
	}
}

#[derive(Clone, Debug)]
pub struct Builder {
	system: System,
	executable: Template,
	env: BTreeMap<String, Template>,
	args: Vec<Template>,
	checksum: Option<Checksum>,
	unsafe_: bool,
	network: bool,
	host_paths: Vec<String>,
}

impl Builder {
	#[must_use]
	pub fn new(system: System, executable: Template) -> Self {
		Self {
			system,
			executable,
			env: BTreeMap::new(),
			args: Vec::new(),
			checksum: None,
			unsafe_: false,
			network: false,
			host_paths: Vec::new(),
		}
	}

	#[must_use]
	pub fn system(mut self, system: System) -> Self {
		self.system = system;
		self
	}

	#[must_use]
	pub fn command(mut self, executable: Template) -> Self {
		self.executable = executable;
		self
	}

	#[must_use]
	pub fn env(mut self, env: BTreeMap<String, Template>) -> Self {
		self.env = env;
		self
	}

	#[must_use]
	pub fn args(mut self, args: Vec<Template>) -> Self {
		self.args = args;
		self
	}

	#[must_use]
	pub fn checksum(mut self, checksum: Checksum) -> Self {
		self.checksum = Some(checksum);
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
	pub fn host_paths(mut self, host_paths: Vec<String>) -> Self {
		self.host_paths = host_paths;
		self
	}

	pub fn build(self, tg: &Instance) -> Result<Command> {
		Command::new(
			tg,
			self.system,
			self.executable,
			self.env,
			self.args,
			self.checksum,
			self.unsafe_,
			self.network,
			self.host_paths,
		)
	}
}
