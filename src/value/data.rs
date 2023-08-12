use super::Value;
use crate::{
	artifact::Artifact,
	blob::Blob,
	block::Block,
	bytes::Bytes,
	error::{return_error, Error, Result},
	id::Id,
	instance::Instance,
	operation::Operation,
	path::{Relpath, Subpath},
	placeholder::{self, Placeholder},
	template::{self, Template},
};
use async_recursion::async_recursion;
use byteorder::{ReadBytesExt, WriteBytesExt};
use futures::{
	stream::{FuturesOrdered, FuturesUnordered},
	TryStreamExt,
};
use std::collections::BTreeMap;

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Null(()),

	#[tangram_serialize(id = 1)]
	Bool(bool),

	#[tangram_serialize(id = 2)]
	Number(f64),

	#[tangram_serialize(id = 3)]
	String(String),

	#[tangram_serialize(id = 4)]
	Bytes(Bytes),

	#[tangram_serialize(id = 5)]
	Relpath(Relpath),

	#[tangram_serialize(id = 6)]
	Subpath(Subpath),

	#[tangram_serialize(id = 7)]
	Block(Id),

	#[tangram_serialize(id = 8)]
	Blob(Id),

	#[tangram_serialize(id = 9)]
	Artifact(Id),

	#[tangram_serialize(id = 10)]
	Placeholder(placeholder::Data),

	#[tangram_serialize(id = 11)]
	Template(template::Data),

	#[tangram_serialize(id = 12)]
	Operation(Id),

	#[tangram_serialize(id = 13)]
	Array(Array),

	#[tangram_serialize(id = 14)]
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
		tangram_serialize::to_writer(self, &mut writer)?;

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
		let value = tangram_serialize::from_reader(reader)?;

		Ok(value)
	}
}

impl Value {
	pub fn to_data(&self) -> Data {
		match self {
			Self::Null => Data::Null(()),
			Self::Bool(bool_) => Data::Bool(*bool_),
			Self::Number(number) => Data::Number(*number),
			Self::String(string) => Data::String(string.clone()),
			Self::Bytes(bytes) => Data::Bytes(bytes.clone()),
			Self::Subpath(path) => Data::Subpath(path.clone()),
			Self::Relpath(path) => Data::Relpath(path.clone()),
			Self::Block(block) => Data::Block(block.id()),
			Self::Blob(blob) => Data::Blob(blob.id()),
			Self::Artifact(artifact) => Data::Artifact(artifact.id()),
			Self::Placeholder(placeholder) => Data::Placeholder(placeholder.to_data()),
			Self::Template(template) => Data::Template(template.to_data()),
			Self::Operation(operation) => Data::Operation(operation.id()),
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
			Data::Null(_) => Ok(Self::Null),
			Data::Bool(bool_) => Ok(Self::Bool(bool_)),
			Data::Number(number) => Ok(Self::Number(number)),
			Data::String(string) => Ok(Self::String(string)),
			Data::Bytes(bytes) => Ok(Self::Bytes(bytes)),
			Data::Subpath(path) => Ok(Self::Subpath(path)),
			Data::Relpath(path) => Ok(Self::Relpath(path)),
			Data::Block(id) => Ok(Self::Block(Block::with_id(id))),
			Data::Blob(id) => {
				let blob = Blob::with_block(tg, Block::with_id(id)).await?;
				Ok(Self::Blob(blob))
			},
			Data::Artifact(id) => {
				let artifact = Artifact::with_block(tg, Block::with_id(id)).await?;
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
			Data::Operation(id) => {
				let operation = Operation::with_block(tg, Block::with_id(id)).await?;
				Ok(Self::Operation(operation))
			},
			Data::Array(array) => {
				let array = array
					.into_iter()
					.map(|value| Self::from_data(tg, value))
					.collect::<FuturesOrdered<_>>()
					.try_collect()
					.await?;
				Ok(Self::Array(array))
			},
			Data::Object(map) => {
				let map = map
					.into_iter()
					.map(|(key, value)| async move {
						let value = Self::from_data(tg, value).await?;
						Ok::<_, Error>((key, value))
					})
					.collect::<FuturesUnordered<_>>()
					.try_collect()
					.await?;
				Ok(Self::Object(map))
			},
		}
	}
}
