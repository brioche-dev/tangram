pub use self::{builder::Builder, error::Error};
use crate::{self as tg, checksum::Checksum};
use url::Url;

mod builder;
#[cfg(feature = "build")]
mod download;
mod error;
pub mod unpack;

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Resource {
	/// The URL to download from.
	#[tangram_serialize(id = 0)]
	url: Url,

	/// The format to unpack the download with.
	#[tangram_serialize(id = 1)]
	unpack: Option<unpack::Format>,

	/// A checksum of the downloaded file.
	#[tangram_serialize(id = 2)]
	checksum: Option<Checksum>,

	/// If this flag is set, then the download will succeed without a checksum.
	#[tangram_serialize(id = 3)]
	unsafe_: bool,
}

crate::value!(Resource);

impl Resource {
	#[must_use]
	pub fn new(
		url: Url,
		unpack: Option<unpack::Format>,
		checksum: Option<Checksum>,
		unsafe_: bool,
	) -> Self {
		Self {
			url,
			unpack,
			checksum,
			unsafe_,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<tg::Value> {
		todo!()
	}

	#[must_use]
	pub fn url(&self) -> &Url {
		&self.url
	}

	#[must_use]
	pub fn unpack(&self) -> Option<unpack::Format> {
		self.unpack
	}

	#[must_use]
	pub fn checksum(&self) -> Option<&Checksum> {
		self.checksum.as_ref()
	}

	#[must_use]
	pub fn unsafe_(&self) -> bool {
		self.unsafe_
	}
}
