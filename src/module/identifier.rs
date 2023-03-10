use crate::{
	constants::ROOT_MODULE_FILE_NAME,
	error::{bail, ensure, Context, Result},
	os, package,
	path::Path,
};
use url::Url;

/// A unique identifier for a module.
#[derive(
	Clone,
	PartialOrd,
	Ord,
	PartialEq,
	Eq,
	Hash,
	Debug,
	serde::Serialize,
	serde::Deserialize,
	buffalo::Serialize,
	buffalo::Deserialize,
)]
#[serde(into = "Url", try_from = "Url")]
#[buffalo(into = "Url", try_from = "Url")]
pub enum Identifier {
	// A normal module.
	Normal(Normal),

	// An artifact module.
	Artifact(Artifact),

	// A library module.
	Lib(Lib),
}

// A normal module.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Normal {
	/// The module's source.
	pub source: Source,

	/// The module's path.
	pub path: Path,
}

// An artifact module.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Artifact {
	/// The module's source.
	pub source: Source,

	/// The module's path.
	pub path: Path,
}

// A library module.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Lib {
	/// The module's path.
	pub path: Path,
}

/// The source for a module.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(tag = "kind", content = "value")]
pub enum Source {
	/// A module in a package at a path.
	#[serde(rename = "path")]
	Path(os::PathBuf),

	/// A module in a package instance.
	#[serde(rename = "instance")]
	Instance(package::instance::Hash),
}

impl Identifier {
	#[must_use]
	pub fn for_root_module_in_package_instance(
		package_instance_hash: package::instance::Hash,
	) -> Identifier {
		Identifier::Normal(Normal {
			source: Source::Instance(package_instance_hash),
			path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		})
	}

	#[must_use]
	pub fn for_root_module_in_package_at_path(package_path: &os::Path) -> Identifier {
		Identifier::Normal(Normal {
			source: Source::Path(package_path.to_owned()),
			path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		})
	}
}

impl Identifier {
	pub async fn for_path(path: &os::Path) -> Result<Identifier> {
		// Find the package path by searching the path's ancestors for a root module.
		let mut found = false;
		let mut package_path = path.to_owned();
		while package_path.pop() {
			if os::fs::exists(&package_path.join(ROOT_MODULE_FILE_NAME)).await? {
				found = true;
				break;
			}
		}
		if !found {
			let path = path.display();
			bail!(r#"Could not find the package for path "{path}"."#,);
		}

		// Get the module path by stripping the package path.
		let module_path: Path = path
			.strip_prefix(&package_path)
			.unwrap()
			.to_owned()
			.into_os_string()
			.into_string()
			.ok()
			.context("The module path was not valid UTF-8.")?
			.parse()
			.context("The module path was not a valid path.")?;

		// Create the module identifier.
		let module_identifier = Identifier::Normal(Normal {
			source: Source::Path(package_path),
			path: module_path,
		});

		Ok(module_identifier)
	}
}

impl From<Identifier> for Url {
	fn from(value: Identifier) -> Self {
		match value {
			Identifier::Normal(value) => {
				let data = hex::encode(serde_json::to_string(&value).unwrap());
				format!("tangram://normal/{data}.ts").parse().unwrap()
			},
			Identifier::Artifact(value) => {
				let data = hex::encode(serde_json::to_string(&value).unwrap());
				format!("tangram://artifact/{data}.ts").parse().unwrap()
			},
			Identifier::Lib(Lib { path }) => format!("tangram://lib/{path}").parse().unwrap(),
		}
	}
}

impl TryFrom<Url> for Identifier {
	type Error = anyhow::Error;

	fn try_from(value: Url) -> Result<Self, Self::Error> {
		// Ensure the scheme is "tangram".
		ensure!(
			value.scheme() == "tangram",
			"The URL has an invalid scheme."
		);

		let domain = value.domain().context("The URL must have a domain.")?;

		let identifier = match domain {
			"normal" => {
				// Remove the ".ts" extension.
				let path = value
					.path()
					.strip_prefix('/')
					.context(r#"The path must begin with a "/"."#)?
					.strip_suffix(".ts")
					.context(r#"The path must end in ".ts"."#)?;

				// Deserialize the path as hex.
				let data = hex::decode(path).context("Failed to deserialize the path as hex.")?;

				// Deserialize the data.
				let identifier = serde_json::from_slice(&data)
					.context("Failed to deserialize the identifier.")?;

				Identifier::Normal(identifier)
			},

			"artifact" => {
				// Remove the ".ts" extension.
				let path = value
					.path()
					.strip_prefix('/')
					.context(r#"The path must begin with a "/"."#)?
					.strip_suffix(".ts")
					.context(r#"The path must end in ".ts"."#)?;

				// Deserialize the path as hex.
				let data = hex::decode(path).context("Failed to deserialize the path as hex.")?;

				// Deserialize the data.
				let identifier = serde_json::from_slice(&data)
					.context("Failed to deserialize the identifier.")?;

				Identifier::Artifact(identifier)
			},

			"lib" => {
				// Get the path
				let path = value
					.path()
					.strip_prefix('/')
					.context(r#"The path must begin with a "/"."#)?
					.parse()
					.context("Invalid path.")?;

				Identifier::Lib(Lib { path })
			},

			_ => bail!("The URL has an invalid domain."),
		};

		Ok(identifier)
	}
}

impl std::fmt::Display for Identifier {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let url: Url = self.clone().into();
		write!(f, "{url}")?;
		Ok(())
	}
}

impl std::str::FromStr for Identifier {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let url: Url = s.parse()?;
		let module_identifier = url.try_into()?;
		Ok(module_identifier)
	}
}

impl From<Identifier> for String {
	fn from(value: Identifier) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for Identifier {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}
