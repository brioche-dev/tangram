pub use self::{data::Data, error::Error};
use crate::{
	error::{return_error, Result},
	instance::Instance,
	operation,
	package::{self, Package},
	path::Subpath,
	value::Value,
};
use std::collections::BTreeMap;

#[cfg(feature = "operation_run")]
mod call;
#[cfg(feature = "operation_run")]
mod context;
mod data;
mod error;
#[cfg(feature = "operation_run")]
mod exception;
#[cfg(feature = "operation_run")]
mod isolate;
#[cfg(feature = "operation_run")]
mod module;
mod new;
#[cfg(feature = "operation_run")]
mod state;
#[cfg(feature = "operation_run")]
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

	/// The kind of the function.
	pub kind: Kind,

	/// The name of the function.
	pub name: String,

	/// The environment variables to call the function with.
	pub env: BTreeMap<String, Value>,

	/// The arguments to call the function with.
	pub args: Vec<Value>,
}

#[derive(
	Clone,
	Copy,
	Debug,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[buffalo(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub enum Kind {
	Function,
	Test,
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

impl std::fmt::Display for Kind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let kind = match self {
			Kind::Function => "function",
			Kind::Test => "test",
		};
		write!(f, "{kind}")?;
		Ok(())
	}
}

impl std::str::FromStr for Kind {
	type Err = crate::error::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let kind = match s {
			"function" => Kind::Function,
			"test" => Kind::Test,
			_ => return_error!(r#"Invalid kind "{s}"."#),
		};
		Ok(kind)
	}
}

impl From<Kind> for String {
	fn from(value: Kind) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Kind {
	type Error = crate::error::Error;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		value.parse()
	}
}
