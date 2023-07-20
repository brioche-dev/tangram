use crate::error::{return_error, Error, Result};

#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[tangram_serialize(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub enum System {
	Amd64Linux,
	Arm64Linux,
	Amd64MacOs,
	Arm64MacOs,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[tangram_serialize(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub enum Arch {
	Amd64,
	Arm64,
}

#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[tangram_serialize(into = "String", try_from = "String")]
#[serde(into = "String", try_from = "String")]
pub enum Os {
	Linux,
	MacOs,
}

impl System {
	pub fn host() -> Result<System> {
		let host = if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
			System::Amd64Linux
		} else if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
			System::Arm64Linux
		} else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
			System::Amd64MacOs
		} else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
			System::Arm64MacOs
		} else {
			return_error!("Unsupported host system.");
		};
		Ok(host)
	}

	#[must_use]
	pub fn arch(&self) -> Arch {
		match self {
			System::Amd64Linux | System::Amd64MacOs => Arch::Amd64,
			System::Arm64Linux | System::Arm64MacOs => Arch::Arm64,
		}
	}

	#[must_use]
	pub fn os(&self) -> Os {
		match self {
			System::Amd64Linux | System::Arm64Linux => Os::Linux,
			System::Amd64MacOs | System::Arm64MacOs => Os::MacOs,
		}
	}
}

impl std::fmt::Display for System {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let system = match self {
			System::Amd64Linux => "amd64_linux",
			System::Arm64Linux => "arm64_linux",
			System::Amd64MacOs => "amd64_macos",
			System::Arm64MacOs => "arm64_macos",
		};
		write!(f, "{system}")?;
		Ok(())
	}
}

impl std::str::FromStr for System {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let system = match s {
			"amd64_linux" => System::Amd64Linux,
			"arm64_linux" => System::Arm64Linux,
			"amd64_macos" => System::Amd64MacOs,
			"arm64_macos" => System::Arm64MacOs,
			_ => return_error!(r#"Invalid system "{s}"."#),
		};
		Ok(system)
	}
}

impl From<System> for String {
	fn from(value: System) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for System {
	type Error = Error;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		value.parse()
	}
}

impl std::fmt::Display for Arch {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let arch = match self {
			Arch::Amd64 => "amd64",
			Arch::Arm64 => "arm64",
		};
		write!(f, "{arch}")?;
		Ok(())
	}
}

impl std::str::FromStr for Arch {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let system = match s {
			"amd64" => Arch::Amd64,
			"arm64" => Arch::Arm64,
			_ => return_error!(r#"Invalid arch "{s}"."#),
		};
		Ok(system)
	}
}

impl From<Arch> for String {
	fn from(value: Arch) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Arch {
	type Error = Error;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		value.parse()
	}
}

impl std::fmt::Display for Os {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let os = match self {
			Os::Linux => "linux",
			Os::MacOs => "macos",
		};
		write!(f, "{os}")?;
		Ok(())
	}
}

impl std::str::FromStr for Os {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let os = match s {
			"linux" => Os::Linux,
			"macos" => Os::MacOs,
			_ => return_error!(r#"Invalid os "{s}"."#),
		};
		Ok(os)
	}
}

impl From<Os> for String {
	fn from(value: Os) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Os {
	type Error = Error;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		value.parse()
	}
}
