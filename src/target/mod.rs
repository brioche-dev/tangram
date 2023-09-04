pub use self::error::Error;
use crate::{self as tg, subpath::Subpath};
use std::collections::BTreeMap;

// #[cfg(feature = "server")]
// mod build;
// #[cfg(feature = "server")]
// mod context;
// #[cfg(feature = "server")]
// mod convert;
mod error;
// #[cfg(feature = "server")]
// mod exception;
// #[cfg(feature = "server")]
// mod isolate;
// #[cfg(feature = "server")]
// mod module;
// #[cfg(feature = "server")]
// mod state;
// #[cfg(feature = "server")]
// mod syscall;

crate::value!(Target);

/// A target.
#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Target {
	/// The target's package.
	#[tangram_serialize(id = 0)]
	pub package: tg::Package,

	/// The path to the module in the package where the target is defined.
	#[tangram_serialize(id = 1)]
	pub path: Subpath,

	/// The name of the target.
	#[tangram_serialize(id = 2)]
	pub name: String,

	/// The target's environment variables.
	#[tangram_serialize(id = 3)]
	pub env: BTreeMap<String, tg::Value>,

	/// The target's arguments.
	#[tangram_serialize(id = 4)]
	pub args: Vec<tg::Value>,
}

impl Target {
	#[must_use]
	pub fn new(
		package: tg::Package,
		path: Subpath,
		name: String,
		env: BTreeMap<String, tg::Value>,
		args: Vec<tg::Value>,
	) -> Self {
		Self {
			package,
			path,
			name,
			env,
			args,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<tg::Value> {
		let mut children = vec![];
		children.push(self.package.clone().into());
		children.extend(self.env.values().cloned());
		children.extend(self.args.iter().cloned());
		children
	}

	#[must_use]
	pub fn package(&self) -> &tg::Package {
		&self.package
	}

	#[must_use]
	pub fn path(&self) -> &Subpath {
		&self.path
	}

	#[must_use]
	pub fn name(&self) -> &String {
		&self.name
	}

	#[must_use]
	pub fn env(&self) -> &BTreeMap<String, tg::Value> {
		&self.env
	}

	#[must_use]
	pub fn args(&self) -> &Vec<tg::Value> {
		&self.args
	}
}
