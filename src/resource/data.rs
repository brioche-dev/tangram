use super::{unpack, Resource};
use crate::{block::Block, checksum::Checksum};
use url::Url;

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
	pub url: Url,

	#[tangram_serialize(id = 1)]
	#[serde(default)]
	pub unpack: Option<unpack::Format>,

	#[tangram_serialize(id = 2)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	#[tangram_serialize(id = 3)]
	#[serde(default, rename = "unsafe")]
	pub unsafe_: bool,
}

impl Resource {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			url: self.url.clone(),
			unpack: self.unpack,
			checksum: self.checksum.clone(),
			unsafe_: self.unsafe_,
		}
	}

	#[must_use]
	pub fn from_data(block: Block, data: Data) -> Self {
		Self {
			block,
			url: data.url,
			unpack: data.unpack,
			checksum: data.checksum,
			unsafe_: data.unsafe_,
		}
	}
}
