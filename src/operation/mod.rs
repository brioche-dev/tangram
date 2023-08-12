pub use self::{
	data::Data,
	error::{Error, Result},
};
use crate::{
	block::Block,
	error::return_error,
	id::Id,
	target::{from_v8, FromV8, ToV8},
};
pub use crate::{resource::Resource, target::Target, task::Task};

mod data;
mod error;
#[cfg(feature = "evaluate")]
mod evaluate;
mod get;
mod output;

/// An operation.
#[derive(Clone, Debug)]
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
	pub fn id(&self) -> Id {
		self.block().id()
	}

	#[must_use]
	pub fn block(&self) -> &Block {
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

impl std::cmp::PartialEq for Operation {
	fn eq(&self, other: &Self) -> bool {
		self.id() == other.id()
	}
}

impl std::cmp::Eq for Operation {}

impl std::cmp::PartialOrd for Operation {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.id().partial_cmp(&other.id())
	}
}

impl std::cmp::Ord for Operation {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.id().cmp(&other.id())
	}
}

impl std::hash::Hash for Operation {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl ToV8 for Operation {
	fn to_v8<'a>(
		&self,
		scope: &mut v8::HandleScope<'a>,
	) -> crate::error::Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Resource(resource) => resource.to_v8(scope),
			Self::Target(target) => target.to_v8(scope),
			Self::Task(task) => task.to_v8(scope),
		}
	}
}

impl FromV8 for Operation {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> crate::error::Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg_string = v8::String::new(scope, "tg").unwrap();
		let tg = global.get(scope, tg_string.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let resource_string = v8::String::new(scope, "Resource").unwrap();
		let resource = tg.get(scope, resource_string.into()).unwrap();
		let resource = v8::Local::<v8::Function>::try_from(resource).unwrap();

		let target_string = v8::String::new(scope, "Target").unwrap();
		let target = tg.get(scope, target_string.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		let task_string = v8::String::new(scope, "Task").unwrap();
		let task = tg.get(scope, task_string.into()).unwrap();
		let task = v8::Local::<v8::Function>::try_from(task).unwrap();

		let operation = if value.instance_of(scope, resource.into()).unwrap() {
			Self::Resource(from_v8(scope, value)?)
		} else if value.instance_of(scope, target.into()).unwrap() {
			Self::Target(from_v8(scope, value)?)
		} else if value.instance_of(scope, task.into()).unwrap() {
			Self::Task(from_v8(scope, value)?)
		} else {
			return_error!("Expected a resource, target, or task.")
		};

		Ok(operation)
	}
}
