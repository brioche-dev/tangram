use super::{unpack, Value};
use crate::checksum::Checksum;
use url::Url;

impl Value {
	#[must_use]
	pub fn builder(url: Url) -> Builder {
		Builder::new(url)
	}
}

pub struct Builder {
	url: Url,
	unpack: Option<unpack::Format>,
	checksum: Option<Checksum>,
	unsafe_: bool,
}

impl Builder {
	#[must_use]
	pub fn new(url: Url) -> Self {
		Self {
			url,
			unpack: None,
			checksum: None,
			unsafe_: false,
		}
	}

	#[must_use]
	pub fn unpack(mut self, unpack: unpack::Format) -> Self {
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
		self.unsafe_ = unsafe_;
		self
	}

	#[must_use]
	pub fn build(self) -> Value {
		Value::new(self.url, self.unpack, self.checksum, self.unsafe_)
	}
}
