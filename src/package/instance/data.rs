use super::{Hash, Instance};
use crate::{
	artifact::{self, Artifact},
	error::{return_error, Result},
	package::{dependency, Package},
};
use byteorder::{ReadBytesExt, WriteBytesExt};
use std::collections::BTreeMap;

#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Data {
	#[buffalo(id = 0)]
	pub package_artifact_hash: artifact::Hash,

	#[buffalo(id = 1)]
	pub dependencies: BTreeMap<dependency::Specifier, Hash>,
}

impl Data {
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

	/// Deserialize a package instance.
	pub fn deserialize<R>(mut reader: R) -> Result<Data>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			return_error!(r#"Cannot deserialize a package instance with version "{version}"."#);
		}

		// Deserialize the package instance.
		let package_instance = buffalo::from_reader(reader)?;

		Ok(package_instance)
	}
}

impl Instance {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			package_artifact_hash: self.package.artifact().hash(),
			dependencies: self.dependencies.clone(),
		}
	}

	pub async fn from_data(tg: &crate::instance::Instance, hash: Hash, data: Data) -> Result<Self> {
		let artifact = Artifact::get(tg, data.package_artifact_hash).await?;
		let package = Package::new(artifact, None);
		Ok(Self {
			hash,
			package,
			dependencies: data.dependencies,
		})
	}
}
