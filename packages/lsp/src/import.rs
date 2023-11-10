use tangram_client as tg;
use tangram_error::{return_error, Error, WrapErr};
use url::Url;

/// An import in a module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Import {
	/// An import of a module in the current package, such as `import "./module.tg"`.
	Path(tg::Relpath),

	/// An import of a dependency, such as `import "tangram:std"`.
	Dependency(tg::Dependency),
}

impl std::fmt::Display for Import {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Import::Path(path) => {
				write!(f, "{path}")?;
			},

			Import::Dependency(dependency) => {
				write!(f, "{dependency}")?;
			},
		}
		Ok(())
	}
}

impl std::str::FromStr for Import {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.starts_with('.') {
			// If the string starts with `.`, then parse the string as a relative path.
			let relpath: tg::Relpath = value.parse()?;

			// Ensure the path has a ".tg" extension.
			if relpath.extension() != Some("tg") {
				return_error!(r#"The path "{relpath}" does not have a ".tg" extension."#);
			}

			Ok(Import::Path(relpath))
		} else {
			// Otherwise, parse the string as a URL.
			let url: Url = value
				.parse()
				.wrap_err_with(|| format!(r#"Failed to parse the string "{value}" as a URL."#))?;

			if url.scheme() == "tangram" {
				// Parse the URL's path as a dependency.
				let dependency = url.path().parse().wrap_err_with(|| {
					let path = url.path();
					format!(r#"Failed to parse "{path}" as a dependency."#)
				})?;
				Ok(Import::Dependency(dependency))
			} else {
				return_error!(r#"The URL "{url}" has an invalid scheme."#)
			}
		}
	}
}

impl From<Import> for String {
	fn from(value: Import) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Import {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}
