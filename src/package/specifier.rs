use crate::os;
use anyhow::Result;

/// A reference to a package, either checked out to a path or in the registry.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Specifier {
	/// A reference to a package checked out to a path.
	Path(os::PathBuf),

	/// A reference to a package in the registry.
	Registry(Registry),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Registry {
	/// The name of the package.
	name: String,

	/// The package's version.
	version: Option<String>,
}

impl std::str::FromStr for Specifier {
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Specifier> {
		if value.starts_with('/') || value.starts_with('.') {
			// If the string starts with `/` or `.`, then it is a path specifier.

			// Parse the string as a path specifier.
			let path = os::PathBuf::from_str(value)?;
			Ok(Specifier::Path(path))
		} else {
			// Parse the string as a registry specifier.
			let mut components = value.split('@');
			let name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);
			Ok(Specifier::Registry(Registry { name, version }))
		}
	}
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Specifier::Path(path) => {
				write!(f, "{}", path.display())?;
				Ok(())
			},
			Specifier::Registry(specifier) => {
				write!(f, "{specifier}")?;
				Ok(())
			},
		}
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
			let right = Specifier::Path(os::PathBuf::from(path_specifier));
			assert_eq!(left, right);
		}
	}
}
