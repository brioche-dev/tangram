use anyhow::{bail, Result};

#[derive(
	Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum System {
	#[serde(rename = "amd64_linux", alias = "x86_64_linux")]
	Amd64Linux,
	#[serde(rename = "amd64_macos", alias = "x86_64_macos")]
	Amd64Macos,
	#[serde(rename = "arm64_linux", alias = "aarch64_linux")]
	Arm64Linux,
	#[serde(rename = "arm64_macos", alias = "aarch64_macos")]
	Arm64Macos,
}

impl System {
	pub fn host() -> Result<System> {
		let host = if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
			System::Amd64Linux
		} else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
			System::Amd64Macos
		} else if cfg!(all(target_arch = "aarch64", target_os = "linux")) {
			System::Arm64Linux
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
			System::Amd64Macos => "amd64_macos",
			System::Arm64Linux => "arm64_linux",
			System::Arm64Macos => "arm64_macos",
		};
		write!(f, "{system}")
	}
}
