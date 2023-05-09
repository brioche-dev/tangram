use crate::error::{return_error, Error};
use std::path::Path;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(into = "String", try_from = "String")]
pub enum Format {
	Tar(Option<Compression>),
	Zip,
}

#[derive(Clone, Debug)]
pub enum Compression {
	Bz2,
	Gz,
	Lz,
	Xz,
	Zstd,
}

impl std::fmt::Display for Format {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Format::Tar(compression) => {
				write!(f, ".tar")?;
				if let Some(compression) = compression {
					write!(f, "{compression}")?;
				}
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
		if let Some(compression) = s.strip_prefix(".tar") {
			let compression = if compression.is_empty() {
				None
			} else {
				Some(compression.parse()?)
			};
			Ok(Format::Tar(compression))
		} else if s == ".zip" {
			Ok(Format::Zip)
		} else {
			return_error!("Invalid unpack format.");
		}
	}
}

impl std::fmt::Display for Compression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let string = match self {
			Compression::Bz2 => ".bz2",
			Compression::Gz => ".gz",
			Compression::Lz => ".lz",
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
			".lz" => Ok(Compression::Lz),
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

impl Format {
	#[allow(clippy::case_sensitive_file_extension_comparisons)]
	#[must_use]
	pub fn for_path(path: &Path) -> Option<Format> {
		let path = path.to_str().unwrap();
		if path.ends_with(".tar.bz2") || path.ends_with(".tbz2") {
			Some(Format::Tar(Some(Compression::Bz2)))
		} else if path.ends_with(".tar.gz") || path.ends_with(".tgz") {
			Some(Format::Tar(Some(Compression::Gz)))
		} else if path.ends_with(".tar.lz") || path.ends_with(".tlz") {
			Some(Format::Tar(Some(Compression::Lz)))
		} else if path.ends_with(".tar.xz") || path.ends_with(".txz") {
			Some(Format::Tar(Some(Compression::Xz)))
		} else if path.ends_with(".tar.zstd")
			|| path.ends_with(".tzstd")
			|| path.ends_with(".tar.zst")
			|| path.ends_with(".tzst")
		{
			Some(Format::Tar(Some(Compression::Zstd)))
		} else if path.ends_with(".tar") {
			Some(Format::Tar(None))
		} else if path.ends_with(".zip") {
			Some(Format::Zip)
		} else {
			None
		}
	}
}
