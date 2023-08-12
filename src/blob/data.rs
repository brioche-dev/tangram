use crate::{
	error::{return_error, Result},
	id::Id,
};
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(
	Clone,
	Debug,
	Eq,
	PartialEq,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub sizes: Vec<(Id, u64)>,
}

impl Data {
	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the blob.
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
			return_error!(r#"Cannot deserialize a blob with version "{version}"."#);
		}

		// Deserialize the blob.
		let artifact = tangram_serialize::from_reader(reader)?;

		Ok(artifact)
	}
}
