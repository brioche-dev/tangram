pub use self::data::Data;
use crate::{
	artifact::Artifact, blob::Blob, path::Path, placeholder::Placeholder, template::Template,
};
use std::collections::BTreeMap;

mod data;

/// A value.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Value {
	/// A null value.
	Null(()),

	/// A boolean value.
	Bool(bool),

	/// A number value.
	Number(f64),

	/// A string value.
	String(String),

	/// A bytes value.
	Bytes(Vec<u8>),

	/// A path value.
	Path(Path),

	/// A blob value.
	Blob(Blob),

	/// An artifact value.
	Artifact(Artifact),

	/// A placeholder value.
	Placeholder(Placeholder),

	/// A template value.
	Template(Template),

	/// An array value.
	Array(Array),

	/// An object value.
	Object(Object),
}

pub type Array = Vec<Value>;

pub type Object = BTreeMap<String, Value>;

impl Value {
	#[must_use]
	pub fn as_null(&self) -> Option<&()> {
		if let Self::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_bool(&self) -> Option<&bool> {
		if let Self::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_number(&self) -> Option<&f64> {
		if let Self::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_string(&self) -> Option<&str> {
		if let Self::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_artifact(&self) -> Option<&Artifact> {
		if let Self::Artifact(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_placeholder(&self) -> Option<&Placeholder> {
		if let Self::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_template(&self) -> Option<&Template> {
		if let Self::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_array(&self) -> Option<&Array> {
		if let Self::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_object(&self) -> Option<&Object> {
		if let Self::Object(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Value {
	#[must_use]
	pub fn into_null(self) -> Option<()> {
		if let Self::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_bool(self) -> Option<bool> {
		if let Self::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_number(self) -> Option<f64> {
		if let Self::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_string(self) -> Option<String> {
		if let Self::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_artifact(self) -> Option<Artifact> {
		if let Self::Artifact(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_placeholder(self) -> Option<Placeholder> {
		if let Self::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_template(self) -> Option<Template> {
		if let Self::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_array(self) -> Option<Array> {
		if let Self::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_object(self) -> Option<Object> {
		if let Self::Object(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
