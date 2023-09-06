use crate::{Client, Error, Result};

crate::id!();

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub enum Value {
	Null(crate::Null),
	Bool(crate::Bool),
	Number(crate::Number),
	String(crate::String),
	Bytes(crate::Bytes),
	Relpath(crate::Relpath),
	Subpath(crate::Subpath),
	Blob(crate::Blob),
	Directory(crate::Directory),
	File(crate::File),
	Symlink(crate::Symlink),
	Placeholder(crate::Placeholder),
	Template(crate::Subpath),
	Package(crate::Package),
	Resource(crate::Resource),
	Target(crate::Target),
	Task(crate::Task),
	Array(crate::Array),
	Object(crate::Object),
}

impl Handle {
	#[must_use]
	pub fn with_id(id: Id) -> Self {
		Self(crate::Handle::with_id(id.into()))
	}

	#[must_use]
	pub fn expect_id(&self) -> Id {
		self.0.expect_id().try_into().unwrap()
	}

	pub async fn id(&self, client: &Client) -> Result<Id> {
		Ok(self.0.id(client).await?.try_into().unwrap())
	}

	#[must_use]
	pub fn value(&self) -> Value {
		match self.0.kind() {
			crate::Kind::Null => Value::Null(self.0.clone().try_into().unwrap()),
			crate::Kind::Bool => Value::Bool(self.0.clone().try_into().unwrap()),
			crate::Kind::Number => Value::Number(self.0.clone().try_into().unwrap()),
			crate::Kind::String => Value::String(self.0.clone().try_into().unwrap()),
			crate::Kind::Bytes => Value::Bytes(self.0.clone().try_into().unwrap()),
			crate::Kind::Relpath => Value::Relpath(self.0.clone().try_into().unwrap()),
			crate::Kind::Subpath => Value::Subpath(self.0.clone().try_into().unwrap()),
			crate::Kind::Blob => Value::Blob(self.0.clone().try_into().unwrap()),
			crate::Kind::Directory => Value::Directory(self.0.clone().try_into().unwrap()),
			crate::Kind::File => Value::File(self.0.clone().try_into().unwrap()),
			crate::Kind::Symlink => Value::Symlink(self.0.clone().try_into().unwrap()),
			crate::Kind::Placeholder => Value::Placeholder(self.0.clone().try_into().unwrap()),
			crate::Kind::Template => Value::Template(self.0.clone().try_into().unwrap()),
			crate::Kind::Package => Value::Package(self.0.clone().try_into().unwrap()),
			crate::Kind::Resource => Value::Resource(self.0.clone().try_into().unwrap()),
			crate::Kind::Target => Value::Target(self.0.clone().try_into().unwrap()),
			crate::Kind::Task => Value::Task(self.0.clone().try_into().unwrap()),
			crate::Kind::Array => Value::Array(self.0.clone().try_into().unwrap()),
			crate::Kind::Object => Value::Object(self.0.clone().try_into().unwrap()),
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
		Ok(Self(value))
	}
}

impl From<Handle> for crate::Handle {
	fn from(value: Handle) -> Self {
		value.0
	}
}

impl TryFrom<crate::Handle> for Handle {
	type Error = Error;

	fn try_from(value: crate::Handle) -> Result<Self, Self::Error> {
		Ok(Self(value))
	}
}

macro_rules! impls {
	($t:ty) => {
		impl From<$t> for Handle {
			fn from(value: $t) -> Self {
				Self(value.into())
			}
		}

		impl TryFrom<Handle> for $t {
			type Error = Error;

			fn try_from(value: Handle) -> Result<Self> {
				value.0.try_into()
			}
		}
	};
}

impls!(crate::null::Handle);
impls!(crate::bool::Handle);
impls!(crate::number::Handle);
impls!(crate::string::Handle);
impls!(crate::bytes::Handle);
impls!(crate::relpath::Handle);
impls!(crate::subpath::Handle);
impls!(crate::array::Handle);
impls!(crate::object::Handle);
