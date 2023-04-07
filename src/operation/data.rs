use super::Hash;
use crate::{
	call::{self, Call},
	download::{self, Download},
	error::{return_error, Result},
	instance::Instance,
	process::{self, Process},
};
use byteorder::{ReadBytesExt, WriteBytesExt};

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum Data {
	#[buffalo(id = 0)]
	Call(call::Data),

	#[buffalo(id = 1)]
	Download(download::Data),

	#[buffalo(id = 2)]
	Process(process::Data),
}

impl Data {
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

	pub fn deserialize<R>(mut reader: R) -> Result<Self>
	where
		R: std::io::Read,
	{
		// Read the version.
		let version = reader.read_u8()?;
		if version != 0 {
			return_error!(r#"Cannot deserialize operation with version "{version}"."#);
		}

		// Deserialize the operation.
		let operation = buffalo::from_reader(reader)?;

		Ok(operation)
	}
}

impl super::Operation {
	#[must_use]
	pub fn to_data(&self) -> Data {
		match self {
			Self::Call(call) => Data::Call(call.to_data()),
			Self::Download(download) => Data::Download(download.to_data()),
			Self::Process(process) => Data::Process(process.to_data()),
		}
	}

	pub async fn from_data(tg: &Instance, hash: Hash, data: Data) -> Result<Self> {
		let operation = match data {
			Data::Call(data) => Self::Call(Call::from_data(tg, hash, data).await?),
			Data::Download(data) => Self::Download(Download::from_data(hash, data)),
			Data::Process(data) => Self::Process(Process::from_data(tg, hash, data).await?),
		};
		Ok(operation)
	}
}
