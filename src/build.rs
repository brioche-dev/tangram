use crate::{resource, return_error, target, task, value, Client};

crate::id!();

/// A build handle.
#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

/// A build variant.
#[derive(Clone, Debug)]
pub enum Variant {
	/// A resource.
	Resource(resource::Handle),

	/// A target.
	Target(target::Handle),

	/// A task.
	Task(task::Handle),
}

impl Handle {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(value::Handle::with_id(id.into()))
	}

	#[must_use]
	pub fn expect_id(&self) -> Id {
		self.0.expect_id().try_into().unwrap()
	}

	pub async fn id(&self, client: &Client) -> crate::Result<Id> {
		Ok(self.0.id(client).await?.try_into().unwrap())
	}

	#[must_use]
	pub fn variant(&self) -> Variant {
		match self.0.kind() {
			value::Kind::Resource => Variant::Resource(self.0.clone().try_into().unwrap()),
			value::Kind::Target => Variant::Target(self.0.clone().try_into().unwrap()),
			value::Kind::Task => Variant::Task(self.0.clone().try_into().unwrap()),
			_ => unreachable!(),
		}
	}

	#[must_use]
	pub fn as_resource(&self) -> Option<resource::Handle> {
		match self.0.kind() {
			value::Kind::Resource => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_target(&self) -> Option<target::Handle> {
		match self.0.kind() {
			value::Kind::Target => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}

	#[must_use]
	pub fn as_task(&self) -> Option<task::Handle> {
		match self.0.kind() {
			value::Kind::Task => Some(self.0.clone().try_into().unwrap()),
			_ => None,
		}
	}
}

impl From<Id> for crate::Id {
	fn from(value: Id) -> Self {
		value.0
	}
}

impl TryFrom<crate::Id> for Id {
	type Error = crate::Error;

	fn try_from(value: crate::Id) -> Result<Self, Self::Error> {
		match value.kind() {
			value::Kind::Resource | value::Kind::Target | value::Kind::Task => Ok(Self(value)),
			_ => return_error!("Expected a build ID."),
		}
	}
}

impl From<Handle> for value::Handle {
	fn from(value: Handle) -> Self {
		value.0
	}
}

impl TryFrom<value::Handle> for Handle {
	type Error = crate::Error;

	fn try_from(value: value::Handle) -> Result<Self, Self::Error> {
		match value.kind() {
			value::Kind::Resource | value::Kind::Target | value::Kind::Task => Ok(Self(value)),
			_ => return_error!("Expected a build value."),
		}
	}
}

impl From<resource::Handle> for Handle {
	fn from(value: resource::Handle) -> Self {
		Self(value.into())
	}
}

impl From<target::Handle> for Handle {
	fn from(value: target::Handle) -> Self {
		Self(value.into())
	}
}

impl From<task::Handle> for Handle {
	fn from(value: task::Handle) -> Self {
		Self(value.into())
	}
}
