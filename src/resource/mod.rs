pub use self::{builder::Builder, data::Data, error::Error};
use crate::{block::Block, checksum::Checksum};
use url::Url;

mod builder;
mod data;
#[cfg(feature = "evaluate")]
mod download;
mod error;
mod new;
pub mod unpack;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Resource {
	/// The resource's block.
	block: Block,

	/// The URL to download from.
	url: Url,

	/// The format to unpack the download with.
	#[serde(default)]
	unpack: Option<unpack::Format>,

	/// A checksum of the downloaded file.
	#[serde(default)]
	checksum: Option<Checksum>,

	/// If this flag is set, then the download will succeed without a checksum.
	#[serde(default, rename = "unsafe")]
	unsafe_: bool,
}

impl Resource {
	/// Get the ID.
	#[must_use]
	pub fn block(&self) -> Block {
		self.block
	}
}
