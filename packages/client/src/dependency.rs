use itertools::Itertools;
use tangram_error::{error, return_error};

use crate::{Error, Relpath, Result};

/// A dependency.
#[derive(
	Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
#[serde(into = "String", try_from = "String")]
pub struct Dependency {
	/// The name of the package.
	pub name: Option<String>,

	/// The package's version.
	pub version: Option<String>,

	/// The package's path.
	pub path: Option<Relpath>,

	/// Whether or not this dependency is an island.
	pub island: Option<bool>,
}

impl Dependency {
	#[must_use]
	pub fn with_path(path: Relpath) -> Self {
		Self {
			name: None,
			version: None,
			path: Some(path),
			island: None,
		}
	}

	#[must_use]
	pub fn with_name_and_version(name: String, version: Option<String>) -> Self {
		Self {
			name: Some(name),
			version,
			path: None,
			island: None,
		}
	}
}

impl std::fmt::Display for Dependency {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let name_and_version = [self.name.as_deref(), self.version.as_deref()]
			.into_iter()
			.flatten()
			.join("@");
		let parameters = [
			self.path.as_ref().map(|s| format!("path=./{s}")),
			self.island.as_ref().map(|s| format!("island={s}")),
		]
		.into_iter()
		.flatten()
		.join(",");
		write!(f, "{name_and_version}")?;
		if !parameters.is_empty() {
			write!(f, "?{parameters}")?;
		}
		Ok(())
	}
}

impl std::str::FromStr for Dependency {
	type Err = Error;

	fn from_str(value: &str) -> Result<Dependency> {
		// Syntax: <name>@<version>?path=<path>,island=<boolean>
		let mut components = value.split('?');
		let mut name_and_version = components.next().unwrap().split('@');
		let parameters = components.next().into_iter().flat_map(|s| {
			let parameters = s.split(',').map(|s| -> Result<(&str, &str)> {
				let mut components = s.split('=');
				let name = components.next().unwrap();
				let value = components.next().ok_or(error!("Expected a value."))?;
				Ok((name, value))
			});
			parameters
		});

		let name = match name_and_version.next() {
			Some(name) if name.is_empty() => None,
			Some(name) => Some(name.into()),
			None => None,
		};

		let version = name_and_version.next().map(String::from);
		let mut path = None;
		let mut island = None;

		for parameter in parameters {
			let (name, value) = parameter?;
			match name {
				"path" => {
					path = Some(value.parse()?);
				},
				"island" => {
					island = Some(value.parse().map_err(|_| error!("Expected a boolean."))?);
				},
				name => {
					return_error!("Unknown parameter: {name}.");
				},
			}
		}

		Ok(Dependency {
			name,
			version,
			path,
			island,
		})
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

#[cfg(test)]
mod tests {
	use crate::Dependency;

	#[test]
	fn display() {
		let dependency = Dependency {
			name: Some("foo".into()),
			version: None,
			path: None,
			island: None,
		};
		assert_eq!(dependency.to_string(), "foo");
		let dependency = Dependency {
			name: Some("foo".into()),
			version: Some("1.2.3".into()),
			path: None,
			island: None,
		};
		assert_eq!(dependency.to_string(), "foo@1.2.3");
		let dependency = Dependency {
			name: Some("foo".into()),
			version: None,
			path: None,
			island: None,
		};
		assert_eq!(dependency.to_string(), "foo");
		let dependency = Dependency {
			name: Some("foo".into()),
			version: Some("1.2.3".into()),
			path: Some("./path/to/foo".parse().unwrap()),
			island: None,
		};
		assert_eq!(dependency.to_string(), "foo@1.2.3?path=./path/to/foo");
		let dependency = Dependency {
			name: Some("foo".into()),
			version: Some("1.2.3".into()),
			path: Some("./path/to/foo".parse().unwrap()),
			island: Some(true),
		};
		assert_eq!(
			dependency.to_string(),
			"foo@1.2.3?path=./path/to/foo,island=true"
		);
		let dependency = Dependency {
			name: None,
			version: None,
			path: Some("./path/to/foo".parse().unwrap()),
			island: None,
		};
		assert_eq!(dependency.to_string(), "?path=./path/to/foo");
	}

	#[test]
	fn parse() {
		let dependency = Dependency {
			name: Some("foo".into()),
			version: None,
			path: None,
			island: None,
		};
		assert_eq!(dependency, "foo".parse().unwrap());
		let dependency = Dependency {
			name: Some("foo".into()),
			version: Some("1.2.3".into()),
			path: None,
			island: None,
		};
		assert_eq!(dependency, "foo@1.2.3".parse().unwrap());
		let dependency = Dependency {
			name: Some("foo".into()),
			version: None,
			path: None,
			island: None,
		};
		assert_eq!(dependency, "foo".parse().unwrap());
		let dependency = Dependency {
			name: Some("foo".into()),
			version: Some("1.2.3".into()),
			path: Some("./path/to/foo".parse().unwrap()),
			island: None,
		};
		assert_eq!(dependency, "foo@1.2.3?path=./path/to/foo".parse().unwrap());
		let dependency = Dependency {
			name: Some("foo".into()),
			version: Some("1.2.3".into()),
			path: Some("./path/to/foo".parse().unwrap()),
			island: Some(true),
		};
		assert_eq!(
			dependency,
			"foo@1.2.3?path=./path/to/foo,island=true".parse().unwrap()
		);
		let dependency = Dependency {
			name: None,
			version: None,
			path: Some("./path/to/foo".parse().unwrap()),
			island: None,
		};
		assert_eq!(dependency, "?path=./path/to/foo".parse().unwrap());
	}
}
