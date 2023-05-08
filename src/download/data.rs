use crate::{checksum::Checksum, operation};
use url::Url;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Data {
	#[buffalo(id = 0)]
	pub url: Url,

	#[buffalo(id = 1)]
	#[serde(default)]
	pub unpack: bool,

	#[buffalo(id = 2)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 3)]
	#[serde(default, rename = "unsafe")]
	pub unsafe_: bool,
}

impl super::Download {
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
	pub fn from_data(hash: operation::Hash, data: Data) -> Self {
		Self {
			hash,
			url: data.url,
			unpack: data.unpack,
			checksum: data.checksum,
			unsafe_: data.unsafe_,
		}
	}
}
