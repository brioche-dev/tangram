use crate::{checksum::Checksum, system::System};
use std::collections::BTreeMap;

impl Task {
	#[must_use]
	pub fn builder(host: System, executable: Template) -> Builder {
		Builder::new(host, executable)
	}
}

#[derive(Clone, Debug)]
pub struct Builder {
	host: System,
	executable: Template,
	env: BTreeMap<String, Template>,
	args: Vec<Template>,
	checksum: Option<Checksum>,
	unsafe_: bool,
	network: bool,
}

impl Builder {
	#[must_use]
	pub fn new(host: System, executable: Template) -> Self {
		Self {
			host,
			executable,
			env: BTreeMap::new(),
			args: Vec::new(),
			checksum: None,
			unsafe_: false,
			network: false,
		}
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
	pub fn build(self) -> Task {
		Task::new(
			self.host,
			self.executable,
			self.env,
			self.args,
			self.checksum,
			self.unsafe_,
			self.network,
		)
	}
}
