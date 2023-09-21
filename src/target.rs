use crate::{language::Position, package, subpath::Subpath, target, value, Package, Result};
use std::{collections::BTreeMap, sync::Arc};
use thiserror::Error;
use url::Url;

crate::id!(Target);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Target);

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
	pub env: BTreeMap<String, value::Handle>,

	/// The target's arguments.
	pub args: Vec<value::Handle>,
}

crate::value!(Target);

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
	pub package: package::Id,

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

impl Handle {
	#[must_use]
	pub fn new(
		package: Package,
		path: Subpath,
		name: String,
		env: BTreeMap<String, value::Handle>,
		args: Vec<value::Handle>,
	) -> Self {
		Self::with_value(Value {
			package,
			path,
			name,
			env,
			args,
		})
	}
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
				.map(|(key, id)| (key, value::Handle::with_id(id)))
				.collect(),
			args: data.args.into_iter().map(value::Handle::with_id).collect(),
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
			args: self.args.iter().map(value::Handle::expect_id).collect(),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<value::Handle> {
		let mut children = vec![];
		children.push(self.package.clone().into());
		children.extend(self.env.values().cloned());
		children.extend(self.args.iter().cloned());
		children
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

/// An error from a target.
#[derive(
	Clone,
	Debug,
	Error,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Error {
	#[tangram_serialize(id = 0)]
	pub message: String,
	#[tangram_serialize(id = 1)]
	pub location: Option<Location>,
	#[tangram_serialize(id = 2)]
	pub stack_trace: Option<StackTrace>,
	#[tangram_serialize(id = 3)]
	pub source: Option<Arc<crate::Error>>,
}

/// A stack trace.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct StackTrace {
	#[tangram_serialize(id = 0)]
	pub stack_frames: Vec<StackFrame>,
}

/// A stack frame.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
	#[tangram_serialize(id = 0)]
	pub location: Option<Location>,
}

/// A source location.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Location {
	#[tangram_serialize(id = 0)]
	pub source: Source,
	#[tangram_serialize(id = 1)]
	pub position: Position,
}

/// A source.
#[derive(
	Clone,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Source {
	#[tangram_serialize(id = 0)]
	Global(Option<String>),
	#[tangram_serialize(id = 1)]
	Module(Url),
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// Write the message.
		write!(f, "{}", self.message)?;

		// Write the stack trace.
		if let Some(stack_trace) = &self.stack_trace {
			write!(f, "{stack_trace}")?;
		}

		Ok(())
	}
}

impl std::fmt::Display for StackTrace {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for stack_frame in &self.stack_frames {
			writeln!(f)?;
			write!(f, "  {stack_frame}")?;
		}
		Ok(())
	}
}

impl std::fmt::Display for StackFrame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(location) = &self.location {
			write!(f, "{location}")?;
		} else {
			write!(f, "[unknown]")?;
		}
		Ok(())
	}
}

impl std::fmt::Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let source = &self.source;
		let line = self.position.line + 1;
		let character = self.position.character + 1;
		write!(f, "{source}:{line}:{character}")?;
		Ok(())
	}
}

impl std::fmt::Display for Source {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Source::Global(path) => {
				let path = path.as_deref().unwrap_or("[unknown]");
				write!(f, "global:{path}")?;
			},

			Source::Module(module) => {
				write!(f, "{module}")?;
			},
		}
		Ok(())
	}
}
