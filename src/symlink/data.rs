use super::Symlink;
use crate::{
	block::Block,
	error::Result,
	instance::Instance,
	template::{self, Template},
};

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub target: template::Data,
}

impl Symlink {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let target = self.target.to_data();
		Data { target }
	}

	pub async fn from_data(tg: &Instance, block: Block, data: Data) -> Result<Self> {
		let target = Template::from_data(tg, data.target).await?;
		Ok(Self { block, target })
	}
}
