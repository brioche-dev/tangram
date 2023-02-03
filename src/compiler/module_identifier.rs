use crate::{package::PackageHash, util::path_exists};
use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use std::path::{Path, PathBuf};
use url::Url;

pub const TANGRAM_INTERNAL_SCHEME: &str = "tangram-internal";
pub const TANGRAM_SCHEME: &str = "tangram";

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "Url", try_from = "Url")]
pub enum ModuleIdentifier {
	Lib {
		path: Utf8PathBuf,
	},

	Hash {
		package_hash: PackageHash,
		module_path: Utf8PathBuf,
	},

	Path {
		package_path: PathBuf,
		module_path: Utf8PathBuf,
	},
}

impl ModuleIdentifier {
	#[must_use]
	pub fn new_lib(path: Utf8PathBuf) -> ModuleIdentifier {
		ModuleIdentifier::Lib { path }
	}

	#[must_use]
	pub fn new_hash(package_hash: PackageHash, module_path: Utf8PathBuf) -> ModuleIdentifier {
		ModuleIdentifier::Hash {
			package_hash,
			module_path,
		}
	}

	#[must_use]
	pub fn new_path(package_path: PathBuf, module_path: Utf8PathBuf) -> ModuleIdentifier {
		ModuleIdentifier::Path {
			package_path,
			module_path,
		}
	}
}

impl ModuleIdentifier {
	pub async fn for_path(path: &Path) -> Result<ModuleIdentifier> {
		// Find the package path by searching the path's ancestors for a manifest.
		let mut found = false;
		let mut package_path = path.to_owned();
		while package_path.pop() {
			if path_exists(&package_path.join("tangram.json")).await? {
				found = true;
				break;
			}
		}
		if !found {
			bail!("Could not find package for path {}", path.display());
		}

		// Get the module path by stripping the package path.
		let module_path = path
			.strip_prefix(&package_path)
			.unwrap()
			.to_owned()
			.try_into()?;

		// Create the module identifier.
		let module_identifier = ModuleIdentifier::new_path(package_path, module_path);
		Ok(module_identifier)
	}
}

impl TryFrom<Url> for ModuleIdentifier {
	type Error = anyhow::Error;

	fn try_from(value: Url) -> Result<Self, Self::Error> {
		let domain = value.domain().context("The URL must have a domain.")?;
		let module_identifier = match domain {
			"lib" => {
				let path = value.path().into();
				ModuleIdentifier::Lib { path }
			},

			"hash" => {
				let package_hash = value
					.path()
					.strip_prefix('/')
					.unwrap()
					.parse()
					.context("Failed to parse the package hash.")?;
				let module_path = value.query().context("The URL must have a query.")?.into();
				ModuleIdentifier::Hash {
					package_hash,
					module_path,
				}
			},

			"path" => {
				let package_path = value.path().into();
				let module_path = value.query().context("The URL must have a query.")?.into();
				ModuleIdentifier::Path {
					package_path,
					module_path,
				}
			},

			_ => bail!(r#"Invalid URL "{value}"."#),
		};

		Ok(module_identifier)
	}
}

impl From<ModuleIdentifier> for Url {
	fn from(value: ModuleIdentifier) -> Self {
		match value {
			ModuleIdentifier::Lib { path } => {
				format!("{TANGRAM_INTERNAL_SCHEME}://lib{path}")
			},

			ModuleIdentifier::Hash {
				package_hash,
				module_path,
			} => {
				format!("{TANGRAM_INTERNAL_SCHEME}://hash/{package_hash}?{module_path}")
			},

			ModuleIdentifier::Path {
				package_path,
				module_path,
			} => {
				let package_path = package_path.display();
				format!("{TANGRAM_INTERNAL_SCHEME}://path{package_path}?{module_path}")
			},
		}
		.parse()
		.unwrap()
	}
}

impl std::fmt::Display for ModuleIdentifier {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let url: Url = self.clone().into();
		write!(f, "{url}")
	}
}

impl std::str::FromStr for ModuleIdentifier {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let url: Url = s.parse()?;
		let module_identifier = url.try_into()?;
		Ok(module_identifier)
	}
}
