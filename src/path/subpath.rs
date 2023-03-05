use itertools::Itertools;

/// A path with only normal components.
#[derive(
	Clone,
	Debug,
	Default,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Serialize,
	serde::Deserialize,
	buffalo::Serialize,
	buffalo::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub struct Subpath {
	components: Vec<String>,
}

impl std::str::FromStr for Subpath {
	type Err = anyhow::Error;

	fn from_str(_string: &str) -> Result<Self, Self::Err> {
		todo!()
	}
}

impl std::fmt::Display for Subpath {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let string = self.components.iter().join("/");
		write!(f, "{string}")?;
		Ok(())
	}
}

impl TryFrom<String> for Subpath {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Subpath> for String {
	fn from(value: Subpath) -> Self {
		value.to_string()
	}
}
