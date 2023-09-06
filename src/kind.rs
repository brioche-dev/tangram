use crate::error::{return_error, Error};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
	Null,
	Bool,
	Number,
	String,
	Bytes,
	Relpath,
	Subpath,
	Blob,
	Directory,
	File,
	Symlink,
	Placeholder,
	Template,
	Package,
	Resource,
	Target,
	Task,
	Array,
	Object,
}

impl From<Kind> for u8 {
	fn from(value: Kind) -> Self {
		match value {
			Kind::Null => 0,
			Kind::Bool => 1,
			Kind::Number => 2,
			Kind::String => 3,
			Kind::Bytes => 4,
			Kind::Relpath => 5,
			Kind::Subpath => 6,
			Kind::Blob => 7,
			Kind::Directory => 8,
			Kind::File => 9,
			Kind::Symlink => 10,
			Kind::Placeholder => 11,
			Kind::Template => 12,
			Kind::Package => 13,
			Kind::Resource => 14,
			Kind::Target => 15,
			Kind::Task => 16,
			Kind::Array => 17,
			Kind::Object => 18,
		}
	}
}

impl TryFrom<u8> for Kind {
	type Error = Error;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(Kind::Null),
			1 => Ok(Kind::Bool),
			2 => Ok(Kind::Number),
			3 => Ok(Kind::String),
			4 => Ok(Kind::Bytes),
			5 => Ok(Kind::Relpath),
			6 => Ok(Kind::Subpath),
			7 => Ok(Kind::Blob),
			8 => Ok(Kind::Directory),
			9 => Ok(Kind::File),
			10 => Ok(Kind::Symlink),
			11 => Ok(Kind::Placeholder),
			12 => Ok(Kind::Template),
			13 => Ok(Kind::Package),
			14 => Ok(Kind::Resource),
			15 => Ok(Kind::Target),
			16 => Ok(Kind::Task),
			17 => Ok(Kind::Array),
			18 => Ok(Kind::Object),
			_ => return_error!("Invalid kind."),
		}
	}
}

/// Define a value kind.
#[macro_export]
macro_rules! kind {
	($t:ident) => {
		impl From<Id> for $crate::Id {
			fn from(value: Id) -> Self {
				value.0
			}
		}

		impl TryFrom<$crate::Id> for Id {
			type Error = $crate::Error;

			fn try_from(value: $crate::Id) -> Result<Self, Self::Error> {
				match value.kind() {
					$crate::Kind::$t => Ok(Self(value)),
					_ => $crate::return_error!("Unexpected kind."),
				}
			}
		}

		impl From<Handle> for $crate::Handle {
			fn from(value: Handle) -> Self {
				value.0
			}
		}

		impl TryFrom<$crate::Handle> for Handle {
			type Error = $crate::Error;

			fn try_from(value: $crate::Handle) -> Result<Self, Self::Error> {
				match value.kind() {
					$crate::Kind::$t => Ok(Self(value)),
					_ => $crate::return_error!("Unexpected kind."),
				}
			}
		}

		impl From<Value> for $crate::Value {
			fn from(value: Value) -> Self {
				$crate::Value::$t(value)
			}
		}

		impl Handle {
			#[must_use]
			pub fn with_id(id: Id) -> Self {
				Self($crate::Handle::with_id(id.into()))
			}

			#[must_use]
			pub fn with_value(value: Value) -> Self {
				Self($crate::Handle::with_value(value.into()))
			}

			#[must_use]
			pub fn expect_id(&self) -> Id {
				self.0.expect_id().try_into().unwrap()
			}

			pub async fn id(&self, client: &$crate::Client) -> $crate::Result<Id> {
				Ok(self.0.id(client).await?.try_into().unwrap())
			}

			pub async fn value(&self, client: &$crate::Client) -> $crate::Result<&Value> {
				match self.0.value(client).await? {
					$crate::Value::$t(value) => Ok(value),
					_ => unreachable!(),
				}
			}
		}
	};
}
