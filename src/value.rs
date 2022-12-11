use crate::{artifact::Artifact, checksum::Checksum, hash::Hash, system::System};
use anyhow::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::{collections::BTreeMap, sync::Arc};
use url::Url;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(tag = "type", content = "value")]
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
	Artifact(Artifact),

	#[buffalo(id = 5)]
	#[serde(rename = "package")]
	Package(Package),

	#[buffalo(id = 6)]
	#[serde(rename = "template")]
	Template(Template),

	#[buffalo(id = 7)]
	#[serde(rename = "placeholder")]
	Placeholder(Placeholder),

	#[buffalo(id = 8)]
	#[serde(rename = "array")]
	Array(Array),

	#[buffalo(id = 9)]
	#[serde(rename = "map")]
	Map(Map),
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Package {
	#[buffalo(id = 0)]
	pub source: Artifact,

	#[buffalo(id = 1)]
	pub dependencies: BTreeMap<String, Package>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Template {
	#[buffalo(id = 0)]
	pub components: Vec<Hash>,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Placeholder {
	#[buffalo(id = 0)]
	pub name: String,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Download {
	#[buffalo(id = 0)]
	pub url: Url,

	#[buffalo(id = 1)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 2)]
	pub unpack: bool,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Process {
	#[buffalo(id = 0)]
	pub system: System,

	#[buffalo(id = 1)]
	pub working_directory: Hash,

	#[buffalo(id = 2)]
	pub env: Hash,

	#[buffalo(id = 3)]
	pub command: Hash,

	#[buffalo(id = 4)]
	pub args: Hash,

	#[buffalo(id = 5)]
	#[serde(default)]
	pub network: bool,

	#[buffalo(id = 6)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 7)]
	#[serde(default, rename = "unsafe")]
	pub is_unsafe: bool,
}

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Target {
	#[buffalo(id = 0)]
	pub package: Hash,

	#[buffalo(id = 1)]
	pub name: String,

	#[buffalo(id = 2)]
	pub args: Hash,
}

pub type Array = Vec<Hash>;

pub type Map = BTreeMap<Arc<str>, Hash>;

impl Value {
	pub fn deserialize<R>(mut reader: R) -> Result<Value>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize expression with version "{version}"."#);
		}

		// Deserialize the expression.
		let expression = buffalo::from_reader(reader)?;

		Ok(expression)
	}

	pub fn deserialize_from_slice(slice: &[u8]) -> Result<Value> {
		Value::deserialize(slice)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the expression.
		buffalo::to_writer(self, &mut writer)?;

		Ok(())
	}

	#[must_use]
	pub fn serialize_to_vec(&self) -> Vec<u8> {
		let mut data = Vec::new();
		self.serialize(&mut data).unwrap();
		data
	}

	#[must_use]
	pub fn serialize_to_vec_and_hash(&self) -> (Hash, Vec<u8>) {
		let data = self.serialize_to_vec();
		let hash = Hash::new(&data);
		(hash, data)
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		let data = self.serialize_to_vec();
		Hash::new(&data)
	}
}

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
	pub fn as_package(&self) -> Option<&Package> {
		if let Value::Package(v) = self {
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
	pub fn as_placeholder(&self) -> Option<&Placeholder> {
		if let Value::Placeholder(v) = self {
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
	pub fn into_package(self) -> Option<Package> {
		if let Value::Package(v) = self {
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
	pub fn into_placeholder(self) -> Option<Placeholder> {
		if let Value::Placeholder(v) = self {
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
