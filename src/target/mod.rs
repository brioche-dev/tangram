pub use self::{data::Data, error::Error};
use crate::{
	block::Block, error::Result, instance::Instance, package::Package, path::Subpath, value::Value,
};
use std::collections::BTreeMap;

#[cfg(feature = "evaluate")]
mod build;
#[cfg(feature = "evaluate")]
mod context;
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
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
	/// The target's block.
	block: Block,

	/// The target's package.
	package: Block,

	/// The path to module where the target is defined.
	module_path: Subpath,

	/// The name of the target.
	name: String,

	/// The target's environment variables.
	env: BTreeMap<String, Value>,

	/// The target's arguments.
	args: Vec<Value>,
}

impl Target {
	#[must_use]
	pub fn block(&self) -> Block {
		self.block
	}

	pub async fn package(&self, tg: &Instance) -> Result<Package> {
		Package::get(tg, self.block).await
	}
}
