use anyhow::Result;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Specifier {
	Path(PathBuf),
	Package(Package),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
	pub name: String,
	pub version: Option<String>,
}

impl std::str::FromStr for Specifier {
	type Err = anyhow::Error;
	fn from_str(source: &str) -> Result<Specifier> {
		if source.starts_with('.') || source.starts_with('/') {
			// Parse this as a path specifier.
			let path = PathBuf::from_str(source)?;
			Ok(Specifier::Path(path))
		} else {
			// Parse this as a registry specifier.
			let mut components = source.split('@');
			let name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);
			Ok(Specifier::Package(Package { name, version }))
		}
	}
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match self {
			Specifier::Path(path) => {
				if path.is_absolute() {
					write!(f, "{}", path.display())
				} else {
					write!(f, "./{}", path.display())
				}
			},
			Specifier::Package(Package { name, version }) => {
				if let Some(v) = version {
					write!(f, "{name}@{v}")
				} else {
					write!(f, "{name}")
				}
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_specifier() {
		let path_specifiers = ["./hello", "./", "."];
		for path_specifier in path_specifiers {
			let left: Specifier = path_specifier.parse().unwrap();
			let right = Specifier::Path(PathBuf::from(path_specifier));
			assert_eq!(left, right);
		}

		let left: Specifier = "hello".parse().unwrap();
		let right = Specifier::Package(Package {
			name: "hello".to_owned(),
			version: None,
		});
		assert_eq!(left, right);

		let left: Specifier = "hello@0.0.0".parse().unwrap();
		let right = Specifier::Package(Package {
			name: "hello".to_owned(),
			version: Some("0.0.0".to_owned()),
		});
		assert_eq!(left, right);
	}
}
