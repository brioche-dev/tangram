use crate::{
	self as tg,
	error::{Error, Result},
	Id, Kind,
};

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
#[tangram_serialize(into = "tg::Value", try_from = "tg::Value")]
pub struct Value(tg::Value);

pub enum Any {
	Null(tg::Null),
	Bool(tg::Bool),
	Number(tg::Number),
	String(tg::String),
	Bytes(tg::Bytes),
	Relpath(tg::Relpath),
	Subpath(tg::Subpath),
	Blob(tg::Blob),
	Directory(tg::Directory),
	File(tg::File),
	Symlink(tg::Symlink),
	Placeholder(tg::Placeholder),
	Template(tg::Subpath),
	Package(tg::Package),
	Resource(tg::Resource),
	Target(tg::Target),
	Task(tg::Task),
	Array(tg::Array),
	Object(tg::Object),
}

impl From<Value> for tg::Value {
	fn from(value: Value) -> Self {
		value.0
	}
}

impl TryFrom<tg::Value> for Value {
	type Error = Error;

	fn try_from(value: tg::Value) -> Result<Self, Self::Error> {
		Ok(Self(value))
	}
}

impl tg::Any {
	pub fn with_id(id: Id) -> Result<Self> {
		tg::Value::with_id(id).try_into()
	}
}

macro_rules! impls {
	($t:ty) => {
		impl From<$t> for tg::Any {
			fn from(value: $t) -> Self {
				Self(value.into())
			}
		}

		impl TryFrom<tg::Any> for $t {
			type Error = Error;

			fn try_from(value: tg::Any) -> Result<Self> {
				value.0.try_into()
			}
		}
	};
}

impls!(tg::Null);
impls!(tg::Bool);
impls!(tg::Number);
impls!(tg::String);
impls!(tg::Bytes);
impls!(tg::Relpath);
impls!(tg::Subpath);
impls!(tg::Array);
impls!(tg::Object);

impl tg::Any {
	#[must_use]
	pub fn get(&self) -> Any {
		match self.0.kind() {
			Kind::Null => Any::Null(self.0.clone().try_into().unwrap()),
			Kind::Bool => Any::Bool(self.0.clone().try_into().unwrap()),
			Kind::Number => Any::Number(self.0.clone().try_into().unwrap()),
			Kind::String => Any::String(self.0.clone().try_into().unwrap()),
			Kind::Bytes => Any::Bytes(self.0.clone().try_into().unwrap()),
			Kind::Relpath => Any::Relpath(self.0.clone().try_into().unwrap()),
			Kind::Subpath => Any::Subpath(self.0.clone().try_into().unwrap()),
			Kind::Blob => Any::Blob(self.0.clone().try_into().unwrap()),
			Kind::Directory => Any::Directory(self.0.clone().try_into().unwrap()),
			Kind::File => Any::File(self.0.clone().try_into().unwrap()),
			Kind::Symlink => Any::Symlink(self.0.clone().try_into().unwrap()),
			Kind::Placeholder => Any::Placeholder(self.0.clone().try_into().unwrap()),
			Kind::Template => Any::Template(self.0.clone().try_into().unwrap()),
			Kind::Package => Any::Package(self.0.clone().try_into().unwrap()),
			Kind::Resource => Any::Resource(self.0.clone().try_into().unwrap()),
			Kind::Target => Any::Target(self.0.clone().try_into().unwrap()),
			Kind::Task => Any::Task(self.0.clone().try_into().unwrap()),
			Kind::Array => Any::Array(self.0.clone().try_into().unwrap()),
			Kind::Object => Any::Object(self.0.clone().try_into().unwrap()),
		}
	}
}
