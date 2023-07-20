pub use self::{
	data::Data,
	error::{Error, Result},
};
use crate::block::Block;
pub use crate::{resource::Resource, target::Target, task::Task};

mod data;
mod error;
#[cfg(feature = "evaluate")]
mod evaluate;
mod get;
mod output;

/// An operation.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Operation {
	/// A resource.
	Resource(Resource),

	/// A target.
	Target(Target),

	/// A task.
	Task(Task),
}

impl Operation {
	#[must_use]
	pub fn block(&self) -> Block {
		match self {
			Self::Resource(resource) => resource.block(),
			Self::Target(target) => target.block(),
			Self::Task(task) => task.block(),
		}
	}
}

impl From<Task> for Operation {
	fn from(value: Task) -> Self {
		Self::Task(value)
	}
}

impl From<Target> for Operation {
	fn from(value: Target) -> Self {
		Self::Target(value)
	}
}

impl From<Resource> for Operation {
	fn from(value: Resource) -> Self {
		Self::Resource(value)
	}
}

impl Operation {
	#[must_use]
	pub fn as_resource(&self) -> Option<&Resource> {
		if let Operation::Resource(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_target(&self) -> Option<&Target> {
		if let Operation::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_task(&self) -> Option<&Task> {
		if let Operation::Task(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Operation {
	#[must_use]
	pub fn into_resource(self) -> Option<Resource> {
		if let Operation::Resource(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_target(self) -> Option<Target> {
		if let Operation::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_task(self) -> Option<Task> {
		if let Operation::Task(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
