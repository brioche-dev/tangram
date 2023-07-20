use super::Operation;
use crate::{
	block::Block,
	error::{return_error, Result},
	instance::Instance,
	resource::{self, Resource},
	target::{self, Target},
	task::{self, Task},
};
use byteorder::{ReadBytesExt, WriteBytesExt};

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
	Resource(resource::Data),

	#[tangram_serialize(id = 1)]
	Target(target::Data),

	#[tangram_serialize(id = 2)]
	Task(task::Data),
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
			Self::Resource(resource) => Data::Resource(resource.to_data()),
			Self::Target(target) => Data::Target(target.to_data()),
			Self::Task(task) => Data::Task(task.to_data()),
		}
	}

	pub async fn from_data(tg: &Instance, block: Block, data: Data) -> Result<Self> {
		let operation = match data {
			Data::Resource(data) => Self::Resource(Resource::from_data(block, data)),
			Data::Target(data) => Self::Target(Target::from_data(tg, block, data).await?),
			Data::Task(data) => Self::Task(Task::from_data(tg, block, data).await?),
		};
		Ok(operation)
	}
}
