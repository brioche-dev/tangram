use super::{Artifact, ArtifactHash};
use crate::hash::Hash;
use anyhow::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};

impl Artifact {
	pub fn deserialize<R>(mut reader: R) -> Result<Artifact>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize artifact with version "{version}"."#);
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
	pub fn serialize_to_vec_and_hash(&self) -> (Vec<u8>, ArtifactHash) {
		let data = self.serialize_to_vec();
		let hash = ArtifactHash(Hash::new(&data));
		(data, hash)
	}

	#[must_use]
	pub fn hash(&self) -> ArtifactHash {
		let data = self.serialize_to_vec();
		ArtifactHash(Hash::new(data))
	}
}
