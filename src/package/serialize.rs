use super::{Package, PackageHash};
use crate::hash::Hash;
use anyhow::{bail, Result};
use byteorder::{ReadBytesExt, WriteBytesExt};

impl Package {
	pub fn deserialize<R>(mut reader: R) -> Result<Package>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize package with version "{version}"."#);
		}

		// Deserialize the package.
		let package = buffalo::from_reader(reader)?;

		Ok(package)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the package.
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
	pub fn serialize_to_vec_and_hash(&self) -> (Vec<u8>, PackageHash) {
		let data = self.serialize_to_vec();
		let hash = PackageHash(Hash::new(&data));
		(data, hash)
	}

	#[must_use]
	pub fn hash(&self) -> PackageHash {
		let data = self.serialize_to_vec();
		PackageHash(Hash::new(data))
	}
}
