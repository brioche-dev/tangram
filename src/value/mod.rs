use crate::{artifact, placeholder::Placeholder, template::Template};
use std::collections::BTreeMap;

mod serialize;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Value {
	#[buffalo(id = 0)]
	#[serde(rename = "null")]
	Null(()),

	#[buffalo(id = 1)]
	#[serde(rename = "bool")]
	Bool(bool),

	#[buffalo(id = 2)]
	#[serde(rename = "number")]
	Number(f64),

	#[buffalo(id = 3)]
	#[serde(rename = "string")]
	String(String),

	#[buffalo(id = 4)]
	#[serde(rename = "artifact")]
	Artifact(artifact::Hash),

	#[buffalo(id = 5)]
	#[serde(rename = "placeholder")]
	Placeholder(Placeholder),

	#[buffalo(id = 6)]
	#[serde(rename = "template")]
	Template(Template),

	#[buffalo(id = 7)]
	#[serde(rename = "array")]
	Array(Array),

	#[buffalo(id = 8)]
	#[serde(rename = "map")]
	Map(Map),
}

pub type Array = Vec<Value>;

pub type Map = BTreeMap<String, Value>;

impl Value {
	#[must_use]
	pub fn as_null(&self) -> Option<&()> {
		if let Value::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_bool(&self) -> Option<&bool> {
		if let Value::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_number(&self) -> Option<&f64> {
		if let Value::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_string(&self) -> Option<&str> {
		if let Value::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_artifact(&self) -> Option<&artifact::Hash> {
		if let Value::Artifact(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_placeholder(&self) -> Option<&Placeholder> {
		if let Value::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_template(&self) -> Option<&Template> {
		if let Value::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_array(&self) -> Option<&Array> {
		if let Value::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_map(&self) -> Option<&Map> {
		if let Value::Map(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Value {
	#[must_use]
	pub fn into_null(self) -> Option<()> {
		if let Value::Null(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_bool(self) -> Option<bool> {
		if let Value::Bool(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_number(self) -> Option<f64> {
		if let Value::Number(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_string(self) -> Option<String> {
		if let Value::String(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_artifact(self) -> Option<artifact::Hash> {
		if let Value::Artifact(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_placeholder(self) -> Option<Placeholder> {
		if let Value::Placeholder(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_template(self) -> Option<Template> {
		if let Value::Template(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_array(self) -> Option<Array> {
		if let Value::Array(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_map(self) -> Option<Map> {
		if let Value::Map(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
