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
}

impl Dependency {
	#[must_use]
	pub fn with_path(path: Relpath) -> Self {
		Self {
			name: None,
			version: None,
			path: Some(path),
		}
	}

	#[must_use]
	pub fn with_name_and_version(name: String, version: Option<String>) -> Self {
		Self {
			name: Some(name),
			version,
			path: None,
		}
	}
}

impl std::fmt::Display for Dependency {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match (&self.name, &self.path) {
			(Some(name), None) => {
				write!(f, "{name}")?;
				if let Some(version) = &self.version {
					write!(f, "@{version}")?;
				}
				Ok(())
			},
			(None, Some(path)) => write!(f, "{path}"),
			_ => unreachable!(),
		}
	}
}

impl std::str::FromStr for Dependency {
	type Err = Error;

	fn from_str(value: &str) -> Result<Dependency> {
		if value.starts_with('.') {
			let path = value.parse()?;
			Ok(Self {
				name: None,
				version: None,
				path: Some(path),
			})
		} else {
			// Otherwise, parse the string as a registry dependency.
			let mut components = value.split('@');
			let name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);

			Ok(Self {
				name: Some(name),
				version,
				path: None,
			})
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
