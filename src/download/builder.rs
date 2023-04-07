use super::Download;
use crate::{checksum::Checksum, error::Result, instance::Instance};
use url::Url;

impl Download {
	#[must_use]
	pub fn builder(url: Url) -> Builder {
		Builder::new(url)
	}
}

pub struct Builder {
	url: Url,
	unpack: Option<bool>,
	checksum: Option<Checksum>,
	is_unsafe: Option<bool>,
}

impl Builder {
	#[must_use]
	pub fn new(url: Url) -> Self {
		Self {
			url,
			unpack: None,
			checksum: None,
			is_unsafe: None,
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
	pub fn is_unsafe(mut self, is_unsafe: bool) -> Self {
		self.is_unsafe = Some(is_unsafe);
		self
	}

	pub async fn build(self, tg: &Instance) -> Result<Download> {
		let url = self.url;
		let unpack = self.unpack.unwrap_or(false);
		let checksum = self.checksum;
		let is_unsafe = self.is_unsafe.unwrap_or(false);
		let download = Download::new(tg, url, unpack, checksum, is_unsafe).await?;
		Ok(download)
	}
}
