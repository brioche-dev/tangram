use crate::{package::PackageHash, util::path_exists};
use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use std::path::{Path, PathBuf};

pub const TANGRAM_INTERNAL_SCHEME: &str = "tangram-internal";
pub const TANGRAM_SCHEME: &str = "tangram";

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "url::Url", try_from = "url::Url")]
pub enum Url {
	Lib {
		path: Utf8PathBuf,
	},

	Core {
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

impl Url {
	#[must_use]
	pub fn new_lib(path: Utf8PathBuf) -> Url {
		Url::Lib { path }
	}

	#[must_use]
	pub fn new_core(path: Utf8PathBuf) -> Url {
		Url::Core { path }
	}

	#[must_use]
	pub fn new_hash(package_hash: PackageHash, module_path: Utf8PathBuf) -> Url {
		Url::Hash {
			package_hash,
			module_path,
		}
	}

	#[must_use]
	pub fn new_path(package_path: PathBuf, module_path: Utf8PathBuf) -> Url {
		Url::Path {
			package_path,
			module_path,
		}
	}
}

impl Url {
	pub async fn for_path(path: &Path) -> Result<Url> {
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

		// Create the URL.
		let url = Url::new_path(package_path, module_path);
		Ok(url)
	}
}

impl TryFrom<url::Url> for Url {
	type Error = anyhow::Error;

	fn try_from(value: url::Url) -> Result<Self, Self::Error> {
		let ty = value.domain().context("The URL must have a domain.")?;
		let url = match ty {
			"lib" => {
				let path = value.path().into();
				Url::Lib { path }
			},

			"core" => {
				let path = value.path().into();
				Url::Core { path }
			},

			"hash" => {
				let package_hash = value
					.path()
					.strip_prefix('/')
					.unwrap()
					.parse()
					.context("Failed to parse the package hash.")?;
				let module_path = value.query().context("The URL must have a query.")?.into();
				Url::Hash {
					package_hash,
					module_path,
				}
			},

			"path" => {
				let package_path = value.path().into();
				let module_path = value.query().context("The URL must have a query.")?.into();
				Url::Path {
					package_path,
					module_path,
				}
			},

			_ => bail!(r#"Invalid URL "{value}"."#),
		};

		Ok(url)
	}
}

impl From<Url> for url::Url {
	fn from(value: Url) -> Self {
		match value {
			Url::Lib { path } => {
				format!("{TANGRAM_INTERNAL_SCHEME}://lib{path}")
			},

			Url::Core { path } => {
				format!("{TANGRAM_INTERNAL_SCHEME}://core{path}")
			},

			Url::Hash {
				package_hash,
				module_path,
			} => {
				format!("{TANGRAM_INTERNAL_SCHEME}://hash/{package_hash}?{module_path}")
			},

			Url::Path {
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

impl std::fmt::Display for Url {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let url: url::Url = self.clone().into();
		write!(f, "{url}")
	}
}

impl std::str::FromStr for Url {
	type Err = anyhow::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let url: url::Url = s.parse()?;
		let url = url.try_into()?;
		Ok(url)
	}
}
