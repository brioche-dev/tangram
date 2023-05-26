pub use self::{data::Data, error::Error};
use crate::{
	error::Result,
	instance::Instance,
	operation,
	package::{self, Package},
	path::Subpath,
	value::Value,
};
use std::collections::BTreeMap;

#[cfg(feature = "v8")]
mod call;

#[cfg(feature = "v8")]
mod context;

mod data;
mod error;

#[cfg(feature = "v8")]
mod exception;

#[cfg(feature = "v8")]
mod isolate;

#[cfg(feature = "v8")]
mod module;
mod new;

#[cfg(feature = "v8")]
mod state;

#[cfg(feature = "v8")]
mod syscall;

/// A function.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
	/// The hash.
	pub hash: operation::Hash,

	/// The hash of the package the function is defined in.
	pub package_hash: package::Hash,

	/// The path to module where the function is defined.
	pub module_path: Subpath,

	/// The name of the function.
	pub name: String,

	/// The environment variables to call the function with.
	pub env: BTreeMap<String, Value>,

	/// The arguments to call the function with.
	pub args: Vec<Value>,
}

impl Function {
	#[must_use]
	pub fn hash(&self) -> operation::Hash {
		self.hash
	}

	pub async fn package(&self, tg: &Instance) -> Result<Package> {
		Package::get(tg, self.package_hash).await
	}
}
