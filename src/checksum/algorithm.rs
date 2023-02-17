use anyhow::bail;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Algorithm {
	Sha256,
	Blake3,
}

impl std::fmt::Display for Algorithm {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let system = match self {
			Algorithm::Sha256 => "sha256",
			Algorithm::Blake3 => "blake3",
		};
		write!(f, "{system}")?;
		Ok(())
	}
}

impl std::str::FromStr for Algorithm {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let system = match s {
			"sha256" => Algorithm::Sha256,
			"blake3" => Algorithm::Blake3,
			_ => bail!(r#"Invalid algorithm "{s}"."#),
		};
		Ok(system)
	}
}

impl TryFrom<String> for Algorithm {
	type Error = anyhow::Error;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Algorithm> for String {
	fn from(value: Algorithm) -> Self {
		value.to_string()
	}
}
