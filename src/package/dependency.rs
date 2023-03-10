use crate::error::Result;
pub use crate::package::specifier::Registry;
use crate::path::Path;

/// A reference from a package to a dependency, either at a path or from the registry.
#[derive(
	Clone,
	Debug,
	PartialOrd,
	Ord,
	PartialEq,
	Eq,
	Hash,
	buffalo::Deserialize,
	buffalo::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub enum Specifier {
	/// A reference to a dependency at a path.
	Path(Path),

	/// A reference to a dependency from the registry.
	Registry(Registry),
}

impl std::str::FromStr for Specifier {
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Specifier> {
		if value.starts_with('.') {
			// If the string starts with `.`, then parse the string as a path.
			let specifier = value.parse()?;
			Ok(Specifier::Path(specifier))
		} else {
			// Otherwise, parse the string as a registry specifier.
			let specifier = value.parse()?;
			Ok(Specifier::Registry(specifier))
		}
	}
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Specifier::Path(path) => {
				write!(f, "{path}")?;
				Ok(())
			},

			Specifier::Registry(specifier) => {
				write!(f, "{specifier}")?;
				Ok(())
			},
		}
	}
}

impl TryFrom<String> for Specifier {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Specifier> for String {
	fn from(value: Specifier) -> Self {
		value.to_string()
	}
}
