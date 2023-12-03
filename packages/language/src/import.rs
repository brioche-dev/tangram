use std::collections::BTreeMap;
use tangram_client as tg;
use tangram_error::{return_error, Error, Result, WrapErr};

/// An import in a module.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Import {
	/// An import of a module, such as `import "./module.tg"`.
	Module(tg::Path),

	/// An import of a dependency, such as `import "tangram:std"`.
	Dependency(tg::Dependency),
}

impl Import {
	pub fn with_specifier_and_attributes(
		specifier: &str,
		attributes: Option<&BTreeMap<String, String>>,
	) -> Result<Self> {
		// Parse the specifier.
		let import = specifier.parse()?;

		// Apply the attributes.
		let import = if let Some(attributes) = attributes {
			match import {
				Self::Module(module) => Self::Module(module),
				Self::Dependency(mut dependency) => {
					let attributes = attributes
						.iter()
						.map(|(key, value)| (key.clone(), value.clone()))
						.collect();
					let params = serde_json::from_value(attributes)
						.wrap_err("Failed to parse the attributes.")?;
					dependency.apply_params(params);
					Self::Dependency(dependency)
				},
			}
		} else {
			import
		};

		Ok(import)
	}
}

impl std::fmt::Display for Import {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Import::Module(path) => {
				write!(f, "{path}")?;
			},

			Import::Dependency(dependency) => {
				write!(f, "tangram:{dependency}")?;
			},
		}
		Ok(())
	}
}

impl std::str::FromStr for Import {
	type Err = Error;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.starts_with('.') {
			let path: tg::Path = value.parse()?;
			if path.extension() != Some("tg") {
				return_error!(r#"The path "{path}" does not have a ".tg" extension."#);
			}
			Ok(Import::Module(path))
		} else if let Some(value) = value.strip_prefix("tangram:") {
			let dependency = value
				.parse()
				.wrap_err_with(|| format!(r#"Failed to parse "{value}" as a dependency."#))?;
			Ok(Import::Dependency(dependency))
		} else {
			return_error!(r#"The import is not valid."#)
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
