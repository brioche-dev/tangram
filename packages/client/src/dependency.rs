use crate::{Error, Relpath, Result};

/// A dependency on a package, either at a path or from the registry.
#[derive(
	Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
#[serde(into = "String", try_from = "String")]
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

#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Registry {
	/// The name.
	pub name: String,

	/// The version.
	pub version: Option<String>,
}

impl std::fmt::Display for Registry {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let name = &self.name;
		write!(f, "{name}")?;
		if let Some(version) = &self.version {
			write!(f, "@{version}")?;
		}
		Ok(())
	}
}

impl std::str::FromStr for Registry {
	type Err = Error;

	fn from_str(value: &str) -> Result<Registry> {
		let mut components = value.split('@');
		let name = components.next().unwrap().to_owned();
		let version = components.next().map(ToOwned::to_owned);
		Ok(Registry { name, version })
	}
}

impl From<Registry> for String {
	fn from(value: Registry) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Registry {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
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
