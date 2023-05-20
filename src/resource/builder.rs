use super::Resource;
use crate::{checksum::Checksum, error::Result, instance::Instance};
use url::Url;

impl Resource {
	#[must_use]
	pub fn builder(url: Url) -> Builder {
		Builder::new(url)
	}
}

pub struct Builder {
	url: Url,
	unpack: Option<bool>,
	checksum: Option<Checksum>,
	unsafe_: Option<bool>,
}

impl Builder {
	#[must_use]
	pub fn new(url: Url) -> Self {
		Self {
			url,
			unpack: None,
			checksum: None,
			unsafe_: None,
		}
	}

	#[must_use]
	pub fn unpack(mut self, unpack: bool) -> Self {
		self.unpack = Some(unpack);
		self
	}

	#[must_use]
	pub fn checksum(mut self, checksum: Checksum) -> Self {
		self.checksum = Some(checksum);
		self
	}

	#[must_use]
	pub fn unsafe_(mut self, unsafe_: bool) -> Self {
		self.unsafe_ = Some(unsafe_);
		self
	}

	pub async fn build(self, tg: &Instance) -> Result<Resource> {
		let url = self.url;
		let unpack = self.unpack.unwrap_or(false);
		let checksum = self.checksum;
		let unsafe_ = self.unsafe_.unwrap_or(false);
		let download = Resource::new(tg, url, unpack, checksum, unsafe_).await?;
		Ok(download)
	}
}
