pub use self::error::Error;
use crate::subpath::Subpath;
use crate::Package;
use crate::{package, target};
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

crate::id!();

crate::kind!(Target);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

/// A target.
#[derive(Clone, Debug)]
pub struct Value {
	/// The target's package.
	pub package: Package,

	/// The path to the module in the package where the target is defined.
	pub path: Subpath,

	/// The name of the target.
	pub name: String,

	/// The target's environment variables.
	pub env: BTreeMap<String, crate::Handle>,

	/// The target's arguments.
	pub args: Vec<crate::Handle>,
}

/// A target.
#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	/// The target's package.
	#[tangram_serialize(id = 0)]
	pub package: crate::package::Id,

	/// The path to the module in the package where the target is defined.
	#[tangram_serialize(id = 1)]
	pub path: Subpath,

	/// The name of the target.
	#[tangram_serialize(id = 2)]
	pub name: String,

	/// The target's environment variables.
	#[tangram_serialize(id = 3)]
	pub env: BTreeMap<String, crate::Id>,

	/// The target's arguments.
	#[tangram_serialize(id = 4)]
	pub args: Vec<crate::Id>,
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		target::Value {
			package: package::Handle::with_id(data.package),
			path: data.path,
			name: data.name,
			env: data
				.env
				.into_iter()
				.map(|(key, id)| (key, crate::Handle::with_id(id)))
				.collect(),
			args: data.args.into_iter().map(crate::Handle::with_id).collect(),
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			package: self.package.expect_id(),
			path: self.path.clone(),
			name: self.name.clone(),
			env: self
				.env
				.iter()
				.map(|(key, value)| (key.clone(), value.expect_id()))
				.collect(),
			args: self.args.iter().map(|arg| arg.expect_id()).collect(),
		}
	}

	#[must_use]
	pub fn new(
		package: Package,
		path: Subpath,
		name: String,
		env: BTreeMap<String, crate::Handle>,
		args: Vec<crate::Handle>,
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
	pub fn children(&self) -> Vec<crate::Handle> {
		let mut children = vec![];
		children.push(self.package.clone().into());
		children.extend(self.env.values().cloned());
		children.extend(self.args.iter().cloned());
		children
	}

	#[must_use]
	pub fn package(&self) -> &Package {
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
	pub fn env(&self) -> &BTreeMap<String, crate::Handle> {
		&self.env
	}

	#[must_use]
	pub fn args(&self) -> &Vec<crate::Handle> {
		&self.args
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		std::iter::once(self.package.into())
			.chain(self.env.values().copied())
			.chain(self.args.iter().copied())
			.collect()
	}
}
