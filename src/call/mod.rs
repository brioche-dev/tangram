pub use self::error::Error;
use crate::{function::Function, value::Value};
use std::collections::BTreeMap;

mod context;
mod error;
mod exception;
mod isolate;
mod module;
mod run;
mod state;
mod syscall;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Call {
	#[buffalo(id = 0)]
	pub function: Function,

	#[buffalo(id = 1)]
	pub context: BTreeMap<String, Value>,

	#[buffalo(id = 2)]
	pub args: Vec<Value>,
}
