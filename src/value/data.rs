use crate::{
	artifact::{self, Artifact},
	blob::{self, Blob},
	error::{return_error, Error, Result},
	instance::Instance,
	path::Path,
	placeholder::{self, Placeholder},
	template::{self, Template},
};
use async_recursion::async_recursion;
use byteorder::{ReadBytesExt, WriteBytesExt};
use futures::future::try_join_all;
use std::collections::BTreeMap;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Data {
	#[buffalo(id = 0)]
	Null(()),

	#[buffalo(id = 1)]
	Bool(bool),

	#[buffalo(id = 2)]
	Number(f64),

	#[buffalo(id = 3)]
	String(String),

	#[buffalo(id = 4)]
	Bytes(Vec<u8>),

	#[buffalo(id = 5)]
	Path(Path),

	#[buffalo(id = 6)]
	Blob(blob::Hash),

	#[buffalo(id = 7)]
	Artifact(artifact::Hash),

	#[buffalo(id = 8)]
	Placeholder(placeholder::Data),

	#[buffalo(id = 9)]
	Template(template::Data),

	#[buffalo(id = 10)]
	Array(Array),

	#[buffalo(id = 11)]
	Object(Object),
}

pub type Array = Vec<Data>;

pub type Object = BTreeMap<String, Data>;

impl Data {
	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the value.
		buffalo::to_writer(self, &mut writer)?;

		Ok(())
	}

	pub fn deserialize<R>(mut reader: R) -> Result<Data>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			return_error!(r#"Cannot deserialize value with version "{version}"."#);
		}

		// Deserialize the value.
		let value = buffalo::from_reader(reader)?;

		Ok(value)
	}
}

impl super::Value {
	pub fn to_data(&self) -> Data {
		match self {
			Self::Null(_) => Data::Null(()),
			Self::Bool(bool_) => Data::Bool(*bool_),
			Self::Number(number) => Data::Number(*number),
			Self::String(string) => Data::String(string.clone()),
			Self::Bytes(bytes) => Data::Bytes(bytes.clone()),
			Self::Path(path) => Data::Path(path.clone()),
			Self::Blob(blob) => Data::Blob(blob.hash()),
			Self::Artifact(artifact) => Data::Artifact(artifact.hash()),
			Self::Placeholder(placeholder) => Data::Placeholder(placeholder.to_data()),
			Self::Template(template) => Data::Template(template.to_data()),
			Self::Array(array) => Data::Array(array.iter().map(Self::to_data).collect()),
			Self::Object(map) => Data::Object(
				map.iter()
					.map(|(key, value)| (key.clone(), value.to_data()))
					.collect(),
			),
		}
	}

	#[async_recursion]
	pub async fn from_data(tg: &'async_recursion Instance, data: Data) -> Result<Self> {
		match data {
			Data::Null(_) => Ok(Self::Null(())),
			Data::Bool(bool_) => Ok(Self::Bool(bool_)),
			Data::Number(number) => Ok(Self::Number(number)),
			Data::String(string) => Ok(Self::String(string)),
			Data::Bytes(bytes) => Ok(Self::Bytes(bytes)),
			Data::Path(path) => Ok(Self::Path(path)),
			Data::Blob(value) => {
				let blob = Blob::with_hash(value);
				Ok(Self::Blob(blob))
			},
			Data::Artifact(hash) => {
				let artifact = Artifact::get(tg, hash).await?;
				Ok(Self::Artifact(artifact))
			},
			Data::Placeholder(placeholder) => {
				let placeholder = Placeholder::from_data(placeholder);
				Ok(Self::Placeholder(placeholder))
			},
			Data::Template(template) => {
				let template = Template::from_data(tg, template).await?;
				Ok(Self::Template(template))
			},
			Data::Array(array) => {
				let array =
					try_join_all(array.into_iter().map(|value| Self::from_data(tg, value))).await?;
				Ok(Self::Array(array))
			},
			Data::Object(map) => {
				let map = try_join_all(map.into_iter().map(|(key, value)| async move {
					let value = Self::from_data(tg, value).await?;
					Ok::<_, Error>((key, value))
				}))
				.await?
				.into_iter()
				.collect();
				Ok(Self::Object(map))
			},
		}
	}
}
