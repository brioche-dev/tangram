use super::Tracker;
use crate::error::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};

impl Tracker {
	pub fn deserialize<R>(mut reader: R) -> Result<Tracker>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize an artifact tracker with version "{version}"."#);
		}

		// Deserialize the artifact tracker.
		let artifact_tracker = buffalo::from_reader(reader)?;

		Ok(artifact_tracker)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the artifact tracker.
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
