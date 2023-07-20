pub use self::data::Data;
use crate::{
	artifact::Artifact,
	blob::Blob,
	block::Block,
	error::Result,
	instance::Instance,
	operation::Operation,
	path::{Relpath, Subpath},
	placeholder::Placeholder,
	template::Template,
};
use std::collections::BTreeMap;

mod data;

/// A value.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Value {
	/// A null value.
	Null,

	/// A boolean value.
	Bool(bool),

	/// A number value.
	Number(f64),

	/// A string value.
	String(String),

	/// A bytes value.
	Bytes(Vec<u8>),

	/// A relpath value.
	Relpath(Relpath),

	/// A subpath value.
	Subpath(Subpath),

	/// A block value.
	Block(Block),

	/// A blob value.
	Blob(Blob),

	/// An artifact value.
	Artifact(Artifact),

	/// A placeholder value.
	Placeholder(Placeholder),

	/// A template value.
	Template(Template),

	/// An operation value.
	Operation(Operation),

	/// An array value.
	Array(Array),

	/// An object value.
	Object(Object),
}

pub type Array = Vec<Value>;

pub type Object = BTreeMap<String, Value>;

impl Value {
	pub fn to_bytes(&self) -> Result<Vec<u8>> {
		let data = self.to_data();
		let mut bytes = Vec::new();
		data.serialize(&mut bytes)?;
		Ok(bytes)
	}

	pub async fn from_bytes(tg: &Instance, bytes: &[u8]) -> Result<Self> {
		let data = Data::deserialize(bytes)?;
		let value = Self::from_data(tg, data).await?;
		Ok(value)
	}
}

impl Value {
	pub fn blocks(&self) -> Vec<Block> {
		match self {
			Self::Null
			| Self::Bool(_)
			| Self::Number(_)
			| Self::String(_)
			| Self::Bytes(_)
			| Self::Relpath(_)
			| Self::Subpath(_)
			| Self::Placeholder(_) => vec![],
			Self::Block(block) => vec![*block],
			Self::Blob(blob) => vec![blob.block()],
			Self::Artifact(artifact) => vec![artifact.block()],
			Self::Template(template) => template.artifacts().map(Artifact::block).collect(),
			Self::Operation(operation) => vec![operation.block()],
			Self::Array(array) => array.iter().flat_map(Self::blocks).collect(),
			Self::Object(object) => object.values().flat_map(Self::blocks).collect(),
		}
	}
}

impl Value {
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
	pub fn as_bytes(&self) -> Option<&[u8]> {
		if let Self::Bytes(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_relpath(&self) -> Option<&Relpath> {
		if let Self::Relpath(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_subpath(&self) -> Option<&Subpath> {
		if let Self::Subpath(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_block(&self) -> Option<&Block> {
		if let Self::Block(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_blob(&self) -> Option<&Blob> {
		if let Self::Blob(v) = self {
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
	pub fn as_operation(&self) -> Option<&Operation> {
		if let Self::Operation(v) = self {
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
	pub fn into_bytes(self) -> Option<Vec<u8>> {
		if let Self::Bytes(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_relpath(self) -> Option<Relpath> {
		if let Self::Relpath(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_subpath(self) -> Option<Subpath> {
		if let Self::Subpath(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_block(self) -> Option<Block> {
		if let Self::Block(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_blob(self) -> Option<Blob> {
		if let Self::Blob(v) = self {
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
	pub fn into_operation(self) -> Option<Operation> {
		if let Self::Operation(v) = self {
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

impl std::fmt::Display for Value {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self {
			Value::Null => f.write_str("null"),
			Value::Bool(value) => f.write_str(&format!("{value}")),
			Value::Number(value) => f.write_str(&format!("{value}")),
			Value::String(value) => f.write_str(&format!(r#""{value}""#)),
			Value::Bytes(value) => f.write_str(&format!(r#"(tg.bytes {}"#, value.len())),
			Value::Relpath(value) => f.write_str(&format!(r#"(tg.relpath "{value}")"#)),
			Value::Subpath(value) => f.write_str(&format!(r#"(tg.subpath "{value}")"#)),
			Value::Block(value) => f.write_str(&format!(r#"(tg.blob {})"#, value.id())),
			Value::Blob(value) => f.write_str(&format!(r#"(tg.blob {})"#, value.block().id())),
			Value::Artifact(value) => f.write_str(&format!("{value}")),
			Value::Placeholder(value) => {
				f.write_str(&format!(r#"(tg.placeholder "${}")"#, value.name,))
			},
			Value::Template(value) => {
				let values = value
					.components()
					.iter()
					.map(|value| format!("${value}"))
					.collect::<String>();
				f.write_str(&format!(r#"(tg.template "${values}")"#))
			},
			Value::Operation(value) => match value {
				Operation::Resource(resource) => {
					f.write_str(&format!(r#"(tg.resource {})"#, resource.block().id()))
				},
				Operation::Target(target) => {
					f.write_str(&format!(r#"(tg.target {})"#, target.block().id()))
				},
				Operation::Task(task) => {
					f.write_str(&format!(r#"(tg.task {})"#, task.block().id()))
				},
			},
			Value::Array(values) => {
				let values = values
					.iter()
					.map(|value| format!("${value}"))
					.collect::<Vec<String>>()
					.join(", ");
				f.write_str(&format!("[{values}]"))
			},
			Value::Object(value) => {
				let value = value
					.iter()
					.map(|(key, value)| format!(r#""{key}": {value}"#))
					.collect::<Vec<String>>()
					.join(", ");
				f.write_str(&format!(r#"{{ {value} }}"#))
			},
		}
	}
}
