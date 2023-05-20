pub use self::{data::Data, error::Error};
use crate::{error::Result, instance::Instance, operation, package, path::Subpath, value::Value};
use std::collections::BTreeMap;

mod call;
mod context;
mod data;
mod error;
mod exception;
mod isolate;
mod module;
mod new;
mod state;
mod syscall;

/// A function.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
	/// The hash.
	pub hash: operation::Hash,

	/// The hash of the package instance of the function.
	pub package_instance_hash: package::instance::Hash,

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

	pub async fn package_instance(&self, tg: &Instance) -> Result<package::Instance> {
		package::Instance::get(tg, self.package_instance_hash).await
	}
}
