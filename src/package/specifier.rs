use super::dependency;
use crate::{
	error::{Error, Result},
	util::fs,
};

/// A reference to a package, either at a path or from the registry.
#[derive(
	Clone,
	Debug,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	buffalo::Serialize,
	buffalo::Deserialize,
	serde::Serialize,
	serde::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
#[buffalo(into = "String", try_from = "String")]
pub enum Specifier {
	/// A reference to a package at a path.
	Path(fs::PathBuf),

	/// A reference to a package from the registry.
	Registry(Registry),
}

#[derive(
	Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct Registry {
	/// The name.
	name: String,

	/// The version.
	version: Option<String>,
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Specifier::Path(path) => {
				let path = path.display();
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

impl std::str::FromStr for Specifier {
	type Err = Error;

	fn from_str(value: &str) -> Result<Specifier> {
		if value.starts_with('/') || value.starts_with('.') {
			// If the string starts with `/` or `.`, then parse the string as a path.
			let specifier = value.parse().map_err(Error::other)?;
			Ok(Specifier::Path(specifier))
		} else {
			// Otherwise, parse the string as a registry specifier.
			let specifier = value.parse()?;
			Ok(Specifier::Registry(specifier))
		}
	}
}

impl From<Specifier> for String {
	fn from(value: Specifier) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Specifier {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
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

impl From<dependency::Specifier> for Specifier {
	fn from(value: dependency::Specifier) -> Self {
		match value {
			dependency::Specifier::Path(path) => Specifier::Path(path.into()),
			dependency::Specifier::Registry(specifier) => Specifier::Registry(specifier),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_specifier() {
		let left: Specifier = "hello".parse().unwrap();
		let right = Specifier::Registry(Registry {
			name: "hello".to_owned(),
			version: None,
		});
		assert_eq!(left, right);

		let left: Specifier = "hello@0.0.0".parse().unwrap();
		let right = Specifier::Registry(Registry {
			name: "hello".to_owned(),
			version: Some("0.0.0".to_owned()),
		});
		assert_eq!(left, right);

		let path_specifiers = [".", "./", "./hello"];
		for path_specifier in path_specifiers {
			let left: Specifier = path_specifier.parse().unwrap();
			let right = Specifier::Path(fs::PathBuf::from(path_specifier));
			assert_eq!(left, right);
		}
	}
}
