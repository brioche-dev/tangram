use super::{Hash, Operation};
use crate::{
	command::{self, Command},
	error::{return_error, Result},
	function::{self, Function},
	instance::Instance,
	resource::{self, Resource},
};
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(
	Clone,
	Debug,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Command(command::Data),

	#[tangram_serialize(id = 1)]
	Function(function::Data),

	#[tangram_serialize(id = 2)]
	Resource(resource::Data),
}

impl Data {
	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the operation.
		tangram_serialize::to_writer(self, &mut writer)?;

		Ok(())
	}

	pub fn deserialize<R>(mut reader: R) -> Result<Self>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			return_error!(r#"Cannot deserialize operation with version "{version}"."#);
		}

		// Deserialize the operation.
		let operation = tangram_serialize::from_reader(reader)?;

		Ok(operation)
	}
}

impl Operation {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Command(command) => Data::Command(command.to_data()),
			Self::Function(function) => Data::Function(function.to_data()),
			Self::Resource(resource) => Data::Resource(resource.to_data()),
		}
	}

	pub async fn from_data(tg: &Instance, hash: Hash, data: Data) -> Result<Self> {
		let operation = match data {
			Data::Command(data) => Self::Command(Command::from_data(tg, hash, data).await?),
			Data::Function(data) => Self::Function(Function::from_data(tg, hash, data).await?),
			Data::Resource(data) => Self::Resource(Resource::from_data(hash, data)),
		};
		Ok(operation)
	}
}
