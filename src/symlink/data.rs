use super::Symlink;
use crate::{
	artifact,
	error::Result,
	instance::Instance,
	template::{self, Template},
};

#[derive(
	Clone,
	Debug,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub target: template::Data,
}

impl Symlink {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			target: self.target.to_data(),
		}
	}

	pub async fn from_data(tg: &Instance, hash: artifact::Hash, data: Data) -> Result<Self> {
		let target = Template::from_data(tg, data.target).await?;
		Ok(Self { hash, target })
	}
}
