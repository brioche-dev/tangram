use super::Value;
use crate::error::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};

impl Value {
	pub fn deserialize<R>(mut reader: R) -> Result<Value>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize value with version "{version}"."#);
		}

		// Deserialize the value.
		let value = buffalo::from_reader(reader)?;

		Ok(value)
	}

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

	#[must_use]
	pub fn serialize_to_vec(&self) -> Vec<u8> {
		let mut data = Vec::new();
		self.serialize(&mut data).unwrap();
		data
	}
}
