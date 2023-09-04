pub use self::error::{Error, Result};
use crate as tg;
use crate::error::return_error;
use crate::Kind;

mod error;
// mod run;

/// A build.
#[derive(Clone, Debug, tangram_serialize::Serialize, tangram_serialize::Deserialize)]
#[tangram_serialize(into = "tg::Value", try_from = "tg::Value")]
pub struct Value(tg::Value);

#[derive(Clone, Debug)]
pub enum Build {
	/// A resource.
	Resource(tg::Resource),

	/// A target.
	Target(tg::Target),

	/// A task.
	Task(tg::Task),
}

impl Value {
	#[must_use]
	pub fn get(&self) -> Build {
		match self.0.kind() {
			Kind::Target => Build::Target(self.0.clone().try_into().unwrap()),
			Kind::Task => Build::Task(self.0.clone().try_into().unwrap()),
			Kind::Resource => Build::Resource(self.0.clone().try_into().unwrap()),
			_ => unreachable!(),
		}
	}
}

impl Value {
	#[must_use]
	pub fn as_target(&self) -> Option<tg::Target> {
		match self.0.kind() {
			Kind::Target => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}
}

impl Value {
	#[must_use]
	pub fn as_task(&self) -> Option<tg::Task> {
		match self.0.kind() {
			Kind::Task => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}
}

impl Value {
	#[must_use]
	pub fn as_resource(&self) -> Option<tg::Resource> {
		match self.0.kind() {
			Kind::Resource => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}
}

impl From<tg::Resource> for Value {
	fn from(value: tg::Resource) -> Self {
		Self(value.into())
	}
}

impl From<tg::Target> for Value {
	fn from(value: tg::Target) -> Self {
		Self(value.into())
	}
}

impl From<tg::Task> for Value {
	fn from(value: tg::Task) -> Self {
		Self(value.into())
	}
}

impl From<Value> for tg::Value {
	fn from(value: Value) -> Self {
		value.0
	}
}

impl TryFrom<tg::Value> for Value {
	type Error = crate::error::Error;

	fn try_from(value: tg::Value) -> std::result::Result<Self, Self::Error> {
		match value.kind() {
			Kind::Target | Kind::Task | Kind::Resource => Ok(Self(value)),
			_ => return_error!("Expected a build value."),
		}
	}
}
