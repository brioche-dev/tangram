use super::dependency;
use crate::{
	error::{return_error, Error, WrapErr},
	path::Path,
};
use url::Url;

/// An import specifier in a Tangram TypeScript module.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Specifier {
	/// A module specifier that refers to an artifact module or a normal module in the current package, such as `import "./src"` or `import "./module.tg"`.
	Path(Path),

	/// A module specifier that refers to a dependency, such as `import "tangram:std"`. See [`dependency::Specifier`].
	Dependency(dependency::Specifier),
}

impl std::fmt::Display for Specifier {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Specifier::Path(path) => {
				write!(f, "{path}")?;
			},

			Specifier::Dependency(specifier) => {
				write!(f, "{specifier}")?;
			},
		}
		Ok(())
	}
}

impl std::str::FromStr for Specifier {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.starts_with('/') || value.starts_with('.') {
			// If the string starts with `/` or `.`, then parse the string as a path.
			let path = value.parse().wrap_err("Failed to parse the path.")?;
			Ok(Specifier::Path(path))
		} else {
			// Otherwise, parse the string as a URL.
			let url: Url = value
				.parse()
				.map_err(Error::other)
				.wrap_err_with(|| format!(r#"Failed to parse the string "{value}" as a URL."#))?;

			if url.scheme() == "tangram" {
				// Parse the URL's path as a package specifier.
				let dependency_specifier =
					url.path().parse().map_err(Error::other).wrap_err_with(|| {
						format!(
							r#"Failed to parse the path "{}" as a module dependency specifier."#,
							url.path()
						)
					})?;
				Ok(Specifier::Dependency(dependency_specifier))
			} else {
				return_error!(r#"The URL "{url}" has an invalid scheme."#)
			}
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
