use super::dependency;
use crate::path::Path;
use anyhow::{bail, Context};
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
	type Err = anyhow::Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.starts_with('/') || value.starts_with('.') {
			// If the string starts with `/` or `.`, then parse the string as a path.
			let path = value.parse().context("Failed to parse the path.")?;
			Ok(Specifier::Path(path))
		} else {
			// Otherwise, parse the string as a URL.
			let url: Url = value
				.parse()
				.with_context(|| format!(r#"Failed to parse the string "{value}" as a URL."#))?;

			match url.scheme() {
				// Handle the `tangram` scheme.
				"tangram" => {
					// Parse the URL's path as a package specifier.
					let dependency_specifier = url.path().parse().with_context(|| {
						format!(
							r#"Failed to parse the path "{}" as a module dependency specifier."#,
							url.path()
						)
					})?;
					Ok(Specifier::Dependency(dependency_specifier))
				},

				// All other schemes are invalid.
				_ => bail!(r#"The URL "{url}" has an invalid scheme."#),
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
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}
