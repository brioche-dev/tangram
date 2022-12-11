use crate::{
	checksum::Checksum,
	hash::Hash,
	system::System,
	value::{Package, Template, Value},
};
use anyhow::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::collections::BTreeMap;
use url::Url;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(tag = "type", content = "value")]
pub enum Operation {
	#[buffalo(id = 0)]
	#[serde(rename = "download")]
	Download(Download),

	#[buffalo(id = 1)]
	#[serde(rename = "process")]
	Process(Process),

	#[buffalo(id = 2)]
	#[serde(rename = "target")]
	Target(Target),
}

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
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
	pub working_directory: Option<Template>,

	#[buffalo(id = 2)]
	pub env: BTreeMap<String, Template>,

	#[buffalo(id = 3)]
	pub command: Template,

	#[buffalo(id = 4)]
	pub args: Vec<Template>,

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
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Target {
	#[buffalo(id = 0)]
	pub package: Package,

	#[buffalo(id = 1)]
	pub name: String,

	#[buffalo(id = 2)]
	pub args: Vec<Value>,
}

impl Operation {
	pub fn deserialize<R>(mut reader: R) -> Result<Operation>
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

	pub fn deserialize_from_slice(slice: &[u8]) -> Result<Operation> {
		Operation::deserialize(slice)
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

impl Operation {
	#[must_use]
	pub fn as_download(&self) -> Option<&Download> {
		if let Operation::Download(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_process(&self) -> Option<&Process> {
		if let Operation::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn as_target(&self) -> Option<&Target> {
		if let Operation::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}
}

impl Operation {
	#[must_use]
	pub fn into_download(self) -> Option<Download> {
		if let Operation::Download(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_process(self) -> Option<Process> {
		if let Operation::Process(v) = self {
			Some(v)
		} else {
			None
		}
	}

	#[must_use]
	pub fn into_target(self) -> Option<Target> {
		if let Operation::Target(v) = self {
			Some(v)
		} else {
			None
		}
	}
}
