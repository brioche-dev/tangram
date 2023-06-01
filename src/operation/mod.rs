pub use self::{
	data::Data,
	error::{Error, Result},
	hash::Hash,
};
pub use crate::{command::Command, function::Function, resource::Resource};

mod children;
mod data;
mod error;
mod get;
mod hash;
mod output;
#[cfg(feature = "operation_run")]
mod run;

/// An operation.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Operation {
	/// A command.
	Command(Command),

	/// A function.
	Function(Function),

	/// A resource.
	Resource(Resource),
}

impl Operation {
	#[must_use]
	pub fn hash(&self) -> Hash {
		match self {
			Self::Command(command) => command.hash(),
			Self::Function(function) => function.hash(),
			Self::Resource(resource) => resource.hash(),
		}
	}
}

impl From<Command> for Operation {
	fn from(value: Command) -> Self {
		Self::Command(value)
	}
}

impl From<Function> for Operation {
	fn from(value: Function) -> Self {
		Self::Function(value)
	}
}

impl From<Resource> for Operation {
	fn from(value: Resource) -> Self {
		Self::Resource(value)
	}
}

impl Operation {
	#[must_use]
	pub fn as_command(&self) -> Option<&Command> {
		if let Operation::Command(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_function(&self) -> Option<&Function> {
		if let Operation::Function(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_resource(&self) -> Option<&Resource> {
		if let Operation::Resource(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Operation {
	#[must_use]
	pub fn into_command(self) -> Option<Command> {
		if let Operation::Command(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_function(self) -> Option<Function> {
		if let Operation::Function(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_resource(self) -> Option<Resource> {
		if let Operation::Resource(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
