use super::{Hash, Operation};
use crate::{
	error::{bail, Result},
	hash,
};
use byteorder::{ReadBytesExt, WriteBytesExt};

impl Operation {
	pub fn deserialize<R>(mut reader: R) -> Result<Operation>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			bail!(r#"Cannot deserialize operation with version "{version}"."#);
		}

		// Deserialize the operation.
		let operation = buffalo::from_reader(reader)?;

		Ok(operation)
	}

	pub fn deserialize_from_slice(slice: &[u8]) -> Result<Operation> {
		Operation::deserialize(slice)
	}

	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the operation.
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
	pub fn serialize_to_vec_and_hash(&self) -> (Hash, Vec<u8>) {
		let data = self.serialize_to_vec();
		let hash = Hash(hash::Hash::new(&data));
		(hash, data)
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		let data = self.serialize_to_vec();
		Hash(hash::Hash::new(data))
	}
}
