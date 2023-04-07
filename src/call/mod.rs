pub use self::{data::Data, error::Error};
use crate::{function::Function, operation, value::Value};
use std::collections::BTreeMap;

mod context;
mod data;
mod error;
mod exception;
mod isolate;
mod module;
mod new;
mod run;
mod state;
mod syscall;

/// A function call.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Call {
	/// The hash of the call.
	pub hash: operation::Hash,

	/// The function to call.
	pub function: Function,

	/// The environment variables to call the function with.
	pub env: BTreeMap<String, Value>,

	/// The arguments to call the function with.
	pub args: Vec<Value>,
}

impl Call {
	/// Get the hash.
	#[must_use]
	pub fn hash(&self) -> operation::Hash {
		self.hash
	}
}
