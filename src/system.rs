use anyhow::{bail, Result};

#[derive(
	Clone,
	Copy,
	Debug,
	Eq,
	Ord,
	PartialEq,
	PartialOrd,
	num_derive::FromPrimitive,
	num_derive::ToPrimitive,
	serde::Serialize,
	serde::Deserialize,
	buffalo::Serialize,
	buffalo::Deserialize,
)]
#[buffalo(into = "String", try_from = "String")]
pub enum System {
	#[serde(rename = "amd64_linux", alias = "x86_64_linux")]
	Amd64Linux = 0,

	#[serde(rename = "arm64_linux", alias = "aarch64_linux")]
	Arm64Linux = 1,

	#[serde(rename = "amd64_macos", alias = "x86_64_macos")]
	Amd64Macos = 2,

	#[serde(rename = "arm64_macos", alias = "aarch64_macos")]
	Arm64Macos = 3,
}

impl System {
	pub fn host() -> Result<System> {
		let host = if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
			System::Amd64Linux
		} else if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
			System::Arm64Linux
		} else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
			System::Amd64Macos
		} else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
			System::Arm64Macos
		} else {
			bail!("Unsupported host system.");
		};
		Ok(host)
	}
}

impl std::fmt::Display for System {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let system = match self {
			System::Amd64Linux => "amd64_linux",
			System::Arm64Linux => "arm64_linux",
			System::Amd64Macos => "amd64_macos",
			System::Arm64Macos => "arm64_macos",
		};
		write!(f, "{system}")?;
		Ok(())
	}
}

impl std::str::FromStr for System {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let system = match s {
			"amd64_linux" => System::Amd64Linux,
			"arm64_linux" => System::Arm64Linux,
			"amd64_macos" => System::Amd64Macos,
			"arm64_macos" => System::Arm64Macos,
			_ => bail!(r#"Invalid system "{s}"."#),
		};
		Ok(system)
	}
}

impl TryFrom<String> for System {
	type Error = anyhow::Error;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<System> for String {
	fn from(value: System) -> Self {
		value.to_string()
	}
}
