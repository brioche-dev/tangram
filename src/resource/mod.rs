pub use self::{builder::Builder, data::Data, error::Error};
use crate::{checksum::Checksum, operation};
use url::Url;

mod builder;
mod data;
mod download;
mod error;
mod new;
mod unpack;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Resource {
	/// The hash.
	hash: operation::Hash,

	/// The URL to download from.
	url: Url,

	/// Whether to unpack the downloaded file.
	#[serde(default)]
	unpack: bool,

	/// A checksum of the downloaded file.
	#[serde(default)]
	checksum: Option<Checksum>,

	/// If this flag is set, then the download will succeed without a checksum.
	#[serde(default, rename = "unsafe")]
	unsafe_: bool,
}

impl Resource {
	/// Get the hash.
	#[must_use]
	pub fn hash(&self) -> operation::Hash {
		self.hash
	}
}
