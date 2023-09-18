use crate::{evaluation, Checksum};
use thiserror::Error;
use url::Url;

crate::id!(Resource);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(Resource);

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

crate::value!(Resource);

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
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

impl Handle {
	#[must_use]
	pub fn new(
		url: Url,
		unpack: Option<unpack::Format>,
		checksum: Option<Checksum>,
		unsafe_: bool,
	) -> Self {
		Self::with_value(Value {
			url,
			unpack,
			checksum,
			unsafe_,
		})
	}
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self {
			url: data.url,
			unpack: data.unpack,
			checksum: data.checksum,
			unsafe_: data.unsafe_,
		}
	}

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
	pub fn build(self) -> Handle {
		Handle::new(self.url, self.unpack, self.checksum, self.unsafe_)
	}
}

pub mod unpack {
	use crate::error::{return_error, Error};

	#[derive(
		Clone,
		Copy,
		Debug,
		serde::Deserialize,
		serde::Serialize,
		tangram_serialize::Deserialize,
		tangram_serialize::Serialize,
	)]
	#[serde(into = "String", try_from = "String")]
	#[tangram_serialize(into = "String", try_from = "String")]
	pub enum Format {
		Tar,
		TarBz2,
		TarGz,
		TarXz,
		TarZstd,
		Zip,
	}

	#[derive(Clone, Copy, Debug)]
	pub enum Compression {
		Bz2,
		Gz,
		Xz,
		Zstd,
	}

	impl std::fmt::Display for Format {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			match self {
				Format::Tar => {
					write!(f, ".tar")?;
				},
				Format::TarBz2 => {
					write!(f, ".tar.bz2")?;
				},
				Format::TarGz => {
					write!(f, ".tar.gz")?;
				},
				Format::TarXz => {
					write!(f, ".tar.xz")?;
				},
				Format::TarZstd => {
					write!(f, ".tar.zstd")?;
				},
				Format::Zip => {
					write!(f, ".zip")?;
				},
			}
			Ok(())
		}
	}

	impl std::str::FromStr for Format {
		type Err = Error;

		fn from_str(s: &str) -> Result<Self, Self::Err> {
			match s {
				".tar" => Ok(Format::Tar),
				".tar.bz2" => Ok(Format::TarBz2),
				".tar.gz" => Ok(Format::TarGz),
				".tar.xz" => Ok(Format::TarXz),
				".tar.zstd" => Ok(Format::TarZstd),
				".zip" => Ok(Format::Zip),
				_ => return_error!("Invalid format."),
			}
		}
	}

	impl std::fmt::Display for Compression {
		fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
			let string = match self {
				Compression::Bz2 => ".bz2",
				Compression::Gz => ".gz",
				Compression::Xz => ".xz",
				Compression::Zstd => ".zstd",
			};
			write!(f, "{string}")?;
			Ok(())
		}
	}

	impl std::str::FromStr for Compression {
		type Err = Error;

		fn from_str(s: &str) -> Result<Self, Self::Err> {
			match s {
				".bz2" => Ok(Compression::Bz2),
				".gz" => Ok(Compression::Gz),
				".xz" => Ok(Compression::Xz),
				".zstd" => Ok(Compression::Zstd),
				_ => return_error!("Invalid compression format."),
			}
		}
	}

	impl From<Format> for String {
		fn from(value: Format) -> Self {
			value.to_string()
		}
	}

	impl TryFrom<String> for Format {
		type Error = Error;

		fn try_from(value: String) -> Result<Self, Self::Error> {
			value.parse()
		}
	}
}

#[derive(
	Clone,
	Debug,
	Error,
	serde::Serialize,
	serde::Deserialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[error(transparent)]
pub struct Error {
	#[tangram_serialize(id = 0)]
	source: Box<evaluation::Error>,
}
