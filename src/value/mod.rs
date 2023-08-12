pub use self::data::Data;
use crate::{
	artifact::Artifact,
	blob::Blob,
	block::Block,
	bytes::Bytes,
	error::Result,
	instance::Instance,
	operation::Operation,
	path::{Relpath, Subpath},
	placeholder::Placeholder,
	target::{FromV8, ToV8},
	template::{self, Template},
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::collections::BTreeMap;

mod data;

/// A value.
#[derive(Clone, Debug)]
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
	Bytes(Bytes),

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
			Self::Block(block) => vec![block.clone()],
			Self::Blob(blob) => vec![blob.block().clone()],
			Self::Artifact(artifact) => vec![artifact.block().clone()],
			Self::Template(template) => {
				template.artifacts().map(Artifact::block).cloned().collect()
			},
			Self::Operation(operation) => vec![operation.block().clone()],
			Self::Array(array) => array.iter().flat_map(Self::blocks).collect(),
			Self::Object(object) => object.values().flat_map(Self::blocks).collect(),
		}
	}

	#[async_recursion]
	pub async fn store(&self, tg: &Instance) -> Result<()> {
		match self {
			Value::Block(block) => {
				block.store(tg).await?;
			},
			Value::Blob(blob) => {
				blob.block().store(tg).await?;
			},
			Value::Artifact(artifact) => {
				artifact.block().store(tg).await?;
			},
			Value::Template(template) => {
				template.store(tg).await?;
			},
			Value::Operation(operation) => {
				operation.block().store(tg).await?;
			},
			Value::Array(array) => {
				array
					.iter()
					.map(|value| value.store(tg))
					.collect::<FuturesUnordered<_>>()
					.try_collect()
					.await?;
			},
			Value::Object(object) => {
				object
					.values()
					.map(|value| value.store(tg))
					.collect::<FuturesUnordered<_>>()
					.try_collect()
					.await?;
			},
			_ => {},
		}
		Ok(())
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
	pub fn as_bytes(&self) -> Option<&Bytes> {
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
	pub fn into_bytes(self) -> Option<Bytes> {
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
			Value::Null => {
				write!(f, "null")?;
			},
			Value::Bool(value) => {
				write!(f, "{value}")?;
			},
			Value::Number(value) => {
				write!(f, "{value}")?;
			},
			Value::String(value) => {
				write!(f, r#""{value}""#)?;
			},
			Value::Bytes(value) => {
				write!(f, r#"(tg.bytes {})"#, value.as_ref().len())?;
			},
			Value::Relpath(value) => {
				write!(f, r#"(tg.relpath {value})"#)?;
			},
			Value::Subpath(value) => {
				write!(f, r#"(tg.subpath {value})"#)?;
			},
			Value::Block(value) => {
				write!(f, r#"(tg.block {})"#, value.id())?;
			},
			Value::Blob(value) => {
				write!(f, r#"(tg.blob {})"#, value.id())?;
			},
			Value::Artifact(value) => {
				write!(f, "{value}")?;
			},
			Value::Placeholder(value) => {
				write!(f, "{value}")?;
			},
			Value::Template(value) => {
				write!(f, r#"(tg.template ""#)?;
				for component in value.components() {
					match component {
						template::Component::String(string) => {
							write!(f, "{string}")?;
						},
						template::Component::Artifact(artifact) => {
							write!(f, r#"${{{artifact}}}"#)?;
						},
						template::Component::Placeholder(placeholder) => {
							write!(f, r#"${{{placeholder}}}"#)?;
						},
					}
				}
				write!(f, r#"")"#)?;
			},
			Value::Operation(value) => match value {
				Operation::Resource(resource) => {
					write!(f, r#"(tg.resource {})"#, resource.id())?;
				},
				Operation::Target(target) => {
					write!(f, r#"(tg.target {})"#, target.id())?;
				},
				Operation::Task(task) => {
					write!(f, r#"(tg.task {})"#, task.id())?;
				},
			},
			Value::Array(value) => {
				let len = value.len();
				write!(f, "[")?;
				for (i, value) in value.iter().enumerate() {
					write!(f, "{value}")?;
					if i < len - 1 {
						write!(f, ", ")?;
					}
				}
				write!(f, "]")?;
			},
			Value::Object(value) => {
				let len = value.len();
				write!(f, "{{")?;
				for (i, (key, value)) in value.iter().enumerate() {
					write!(f, r#""{key}": {value}"#)?;
					if i < len - 1 {
						write!(f, ", ")?;
					}
				}
				write!(f, "}}")?;
			},
		}
		Ok(())
	}
}

impl ToV8 for Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		// match self {
		// 	Value::Null => ().to_v8(scope),
		// 	Value::Bool(value) => value.to_v8(scope),
		// 	Value::Number(value) => value.to_v8(scope),
		// 	Value::String(value) => value.to_v8(scope),
		// 	Value::Bytes(value) => value.to_v8(scope),
		// 	Value::Relpath(value) => value.to_v8(scope),
		// 	Value::Subpath(value) => value.to_v8(scope),
		// 	Value::Block(value) => value.to_v8(scope),
		// 	Value::Blob(value) => value.to_v8(scope),
		// 	Value::Artifact(value) => value.to_v8(scope),
		// 	Value::Placeholder(value) => value.to_v8(scope),
		// 	Value::Template(value) => value.to_v8(scope),
		// 	Value::Operation(value) => value.to_v8(scope),
		// 	Value::Array(value) => value.to_v8(scope),
		// 	Value::Object(value) => value.to_v8(scope),
		// }
		todo!()
	}
}

impl FromV8 for Value {
	fn from_v8<'a>(
		_scope: &mut v8::HandleScope<'a>,
		_value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		todo!()
	}
}
