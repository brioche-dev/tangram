use anyhow::Result;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Specifier {
	Package {
		name: String,
		version: Option<String>,
	},
	Path {
		path: PathBuf,
	},
}

impl std::str::FromStr for Specifier {
	type Err = anyhow::Error;
	fn from_str(source: &str) -> Result<Specifier> {
		if source.starts_with('.') || source.starts_with('/') {
			// Parse as a path specifier.
			let path = PathBuf::from_str(source)?;
			Ok(Specifier::Path { path })
		} else {
			// Parse as a registry specifier.
			let mut components = source.split('@');
			let name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);
			Ok(Specifier::Package { name, version })
		}
	}
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Specifier::Package { name, version } => {
				write!(f, "{name}")?;
				if let Some(version) = version {
					write!(f, "@{version}")?;
				}
				Ok(())
			},
			Specifier::Path { path } => {
				write!(f, "{}", path.display())
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_specifier() {
		let left: Specifier = "hello".parse().unwrap();
		let right = Specifier::Package {
			name: "hello".to_owned(),
			version: None,
		};
		assert_eq!(left, right);

		let left: Specifier = "hello@0.0.0".parse().unwrap();
		let right = Specifier::Package {
			name: "hello".to_owned(),
			version: Some("0.0.0".to_owned()),
		};
		assert_eq!(left, right);

		let path_specifiers = ["./hello", "./", "."];
		for path_specifier in path_specifiers {
			let left: Specifier = path_specifier.parse().unwrap();
			let right = Specifier::Path {
				path: PathBuf::from(path_specifier),
			};
			assert_eq!(left, right);
		}
	}
}
