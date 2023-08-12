pub use self::{
	buffer::Buffer,
	convert::{from_v8, to_v8, FromV8, ToV8},
};
pub use self::{data::Data, error::Error};
use crate::{
	block::Block, error::Result, id::Id, instance::Instance, package::Package, path::Subpath,
	value::Value,
};
use std::collections::BTreeMap;

mod buffer;
#[cfg(feature = "evaluate")]
mod build;
#[cfg(feature = "evaluate")]
mod context;
mod convert;
mod data;
mod error;
#[cfg(feature = "evaluate")]
mod exception;
#[cfg(feature = "evaluate")]
mod isolate;
#[cfg(feature = "evaluate")]
mod module;
mod new;
#[cfg(feature = "evaluate")]
mod state;
#[cfg(feature = "evaluate")]
mod syscall;

/// A target.
#[derive(Clone, Debug)]
pub struct Target {
	/// The target's block.
	block: Block,

	/// The target's package.
	package: Block,

	/// The path to the module in the package where the target is defined.
	path: Subpath,

	/// The name of the target.
	name: String,

	/// The target's environment variables.
	env: BTreeMap<String, Value>,

	/// The target's arguments.
	args: Vec<Value>,
}

impl Target {
	#[must_use]
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
		&self.block
	}

	pub async fn package(&self, tg: &Instance) -> Result<Package> {
		Package::with_block(tg, self.block().clone()).await
	}
}

impl std::cmp::PartialEq for Target {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Target {}

impl std::cmp::PartialOrd for Target {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Target {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Target {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Target {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		todo!()
	}
}

impl FromV8 for Target {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}
