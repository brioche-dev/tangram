pub use crate::package::specifier::Registry;
use crate::{
	error::{Error, Result},
	relpath::Relpath,
};

/// A dependency on a package, either at a path or from the registry.
#[derive(
	Clone,
	Debug,
	Eq,
	Hash,
	Ord,
	PartialEq,
	PartialOrd,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[tangram_serialize(into = "String", try_from = "String")]
pub enum Dependency {
	/// A dependency on a package at a path.
	Path(Relpath),

	/// A dependency on a package from the registry.
	Registry(Registry),
}

impl std::fmt::Display for Dependency {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Dependency::Path(path) => {
				write!(f, "{path}")?;
				Ok(())
			},

			Dependency::Registry(registry) => {
				write!(f, "{registry}")?;
				Ok(())
			},
		}
	}
}

impl std::str::FromStr for Dependency {
	type Err = Error;

	fn from_str(value: &str) -> Result<Dependency> {
		if value.starts_with('.') {
			// If the string starts with `.`, then parse the string as a relative path.
			let path = value.parse()?;
			Ok(Dependency::Path(path))
		} else {
			// Otherwise, parse the string as a registry dependency.
			let registry = value.parse()?;
			Ok(Dependency::Registry(registry))
		}
	}
}

impl TryFrom<String> for Dependency {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl From<Dependency> for String {
	fn from(value: Dependency) -> Self {
		value.to_string()
	}
}
