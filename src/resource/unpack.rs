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
	TarLz,
	TarXz,
	TarZstd,
	Zip,
}

#[derive(Clone, Copy, Debug)]
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
			Format::Tar => {
				write!(f, ".tar")?;
			},
			Format::TarBz2 => {
				write!(f, ".tar.bz2")?;
			},
			Format::TarGz => {
				write!(f, ".tar.gz")?;
			},
			Format::TarLz => {
				write!(f, ".tar.lz")?;
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
			".tar.lz" => Ok(Format::TarLz),
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
