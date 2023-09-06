pub use self::builder::Builder;
use crate::checksum::Checksum;
use url::Url;

mod builder;
// mod download;
// mod error;
pub mod unpack;

crate::id!();

crate::kind!(Resource);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub struct Value {
	/// The URL to download from.
	pub url: Url,

	/// The format to unpack the download with.
	pub unpack: Option<unpack::Format>,

	/// A checksum of the downloaded file.
	pub checksum: Option<Checksum>,

	/// If this flag is set, then the download will succeed without a checksum.
	pub unsafe_: bool,
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Data {
	/// The URL to download from.
	#[tangram_serialize(id = 0)]
	pub url: Url,

	/// The format to unpack the download with.
	#[tangram_serialize(id = 1)]
	pub unpack: Option<unpack::Format>,

	/// A checksum of the downloaded file.
	#[tangram_serialize(id = 2)]
	pub checksum: Option<Checksum>,

	/// If this flag is set, then the download will succeed without a checksum.
	#[tangram_serialize(id = 3)]
	pub unsafe_: bool,
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Value {
			url: data.url,
			unpack: data.unpack,
			checksum: data.checksum,
			unsafe_: data.unsafe_,
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		todo!()
	}

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
	pub fn children(&self) -> Vec<crate::Handle> {
		vec![]
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

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		vec![]
	}
}
