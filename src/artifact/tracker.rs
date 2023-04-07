use crate::{
	artifact,
	error::{return_error, Result},
};
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(Clone, Debug, buffalo::Serialize, buffalo::Deserialize)]
pub struct Tracker {
	#[buffalo(id = 0)]
	pub artifact_hash: artifact::Hash,

	#[buffalo(id = 1)]
	pub timestamp_seconds: u64,

	#[buffalo(id = 2)]
	pub timestamp_nanoseconds: u32,
}

impl Tracker {
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

	pub fn deserialize<R>(mut reader: R) -> Result<Tracker>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			return_error!(r#"Cannot deserialize an artifact tracker with version "{version}"."#);
		}

		// Deserialize the artifact tracker.
		let artifact_tracker = buffalo::from_reader(reader)?;

		Ok(artifact_tracker)
	}
}
