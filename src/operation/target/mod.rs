use crate::{package::PackageHash, value::Value};

mod context;
mod exception;
mod isolate;
mod module;
mod run;
mod syscall;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Target {
	#[buffalo(id = 0)]
	pub package: PackageHash,

	#[buffalo(id = 1)]
	pub name: String,

	#[buffalo(id = 2)]
	pub args: Vec<Value>,
}
