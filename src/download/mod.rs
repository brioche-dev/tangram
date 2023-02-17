use crate::checksum::Checksum;
use url::Url;

mod run;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Download {
	#[buffalo(id = 0)]
	pub url: Url,

	#[buffalo(id = 1)]
	pub unpack: bool,

	#[buffalo(id = 2)]
	pub checksum: Option<Checksum>,

	#[buffalo(id = 3)]
	#[serde(default, rename = "unsafe")]
	pub is_unsafe: bool,
}

pub enum ArchiveFormat {
	Tar(Option<CompressionFormat>),
	Zip,
}

pub enum CompressionFormat {
	Bz2,
	Gz,
	Lz,
	Xz,
	Zstd,
}
