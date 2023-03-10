use super::{Hash, Instance};
use crate::{
	error::{bail, Result},
	hash,
};
use byteorder::{ReadBytesExt, WriteBytesExt};

impl Instance {
	/// Deserialize a package instance.
	pub fn deserialize<R>(mut reader: R) -> Result<Instance>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize a package instance with version "{version}"."#);
		}

		// Deserialize the package instance.
		let package_instance = buffalo::from_reader(reader)?;

		Ok(package_instance)
	}

	/// Serialize a package instance.
	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the package instance.
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
