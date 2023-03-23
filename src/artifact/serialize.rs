use super::{Artifact, Hash};
use crate::{
	error::{return_error, Result},
	hash,
};
use byteorder::{ReadBytesExt, WriteBytesExt};

impl Artifact {
	pub fn deserialize<R>(mut reader: R) -> Result<Artifact>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			return_error!(r#"Cannot deserialize an artifact with version "{version}"."#);
		}

		// Deserialize the artifact.
		let artifact = buffalo::from_reader(reader)?;

		Ok(artifact)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the artifact.
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
	pub fn serialize_to_vec_and_hash(&self) -> (Vec<u8>, Hash) {
		let data = self.serialize_to_vec();
		let hash = Hash(hash::Hash::new(&data));
		(data, hash)
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		let data = self.serialize_to_vec();
		Hash(hash::Hash::new(data))
	}
}
