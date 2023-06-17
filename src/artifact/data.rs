use super::{Artifact, Hash};
use crate::{
	directory::{self, Directory},
	error::{return_error, Result},
	file::{self, File},
	instance::Instance,
	symlink::{self, Symlink},
};
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(
	Clone,
	Debug,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Data {
	#[tangram_serialize(id = 0)]
	Directory(directory::Data),

	#[tangram_serialize(id = 1)]
	File(file::Data),

	#[tangram_serialize(id = 2)]
	Symlink(symlink::Data),
}

impl Data {
	pub fn serialize<W>(&self, mut writer: W) -> Result<()>
	where
		W: std::io::Write,
	{
		// Write the version.
		writer.write_u8(0)?;

		// Write the artifact.
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
			return_error!(r#"Cannot deserialize an artifact with version "{version}"."#);
		}

		// Deserialize the artifact.
		let artifact = tangram_serialize::from_reader(reader)?;

		Ok(artifact)
	}
}

impl Artifact {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Directory(directory) => Data::Directory(directory.to_data()),
			Self::File(file) => Data::File(file.to_data()),
			Self::Symlink(symlink) => Data::Symlink(symlink.to_data()),
		}
	}

	pub async fn from_data(tg: &Instance, hash: Hash, data: Data) -> Result<Self> {
		let artifact = match data {
			Data::Directory(data) => {
				let directory = Directory::from_data(hash, data);
				Self::Directory(directory)
			},
			Data::File(data) => {
				let file = File::from_data(hash, data);
				Self::File(file)
			},
			Data::Symlink(data) => {
				let symlink = Symlink::from_data(tg, hash, data).await?;
				Self::Symlink(symlink)
			},
		};
		Ok(artifact)
	}
}
