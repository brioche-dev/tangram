use crate::{hash::Hash, util::path_exists};
use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use std::path::{Path, PathBuf};

pub const TANGRAM_BUILTINS_SCHEME: &str = "tangram-builtins";
pub const TANGRAM_LIB_SCHEME: &str = "tangram-lib";
pub const TANGRAM_PACKAGE_MODULE_SCHEME: &str = "tangram-package-module";
pub const TANGRAM_PACKAGE_TARGETS_SCHEME: &str = "tangram-package-targets";
pub const TANGRAM_PATH_MODULE_SCHEME: &str = "tangram-path-module";
pub const TANGRAM_PATH_TARGETS_SCHEME: &str = "tangram-path-targets";
pub const TANGRAM_SCHEME: &str = "tangram";

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "url::Url", try_from = "url::Url")]
pub enum Url {
	Builtins {
		path: Utf8PathBuf,
	},
	Lib {
		path: Utf8PathBuf,
	},
	PackageModule {
		package_hash: Hash,
		module_path: Utf8PathBuf,
	},
	PackageTargets {
		package_hash: Hash,
	},
	PathModule {
		package_path: PathBuf,
		module_path: Utf8PathBuf,
	},
	PathTargets {
		package_path: PathBuf,
	},
}

impl Url {
	#[must_use]
	pub fn new_builtins(path: Utf8PathBuf) -> Url {
		Url::Builtins { path }
	}

	#[must_use]
	pub fn new_package_module(package_hash: Hash, module_path: Utf8PathBuf) -> Url {
		Url::PackageModule {
			package_hash,
			module_path,
		}
	}

	#[must_use]
	pub fn new_package_targets(package_hash: Hash) -> Url {
		Url::PackageTargets { package_hash }
	}

	#[must_use]
	pub fn new_path_module(package_path: PathBuf, module_path: Utf8PathBuf) -> Url {
		Url::PathModule {
			package_path,
			module_path,
		}
	}

	#[must_use]
	pub fn new_path_targets(package_path: PathBuf) -> Url {
		Url::PathTargets { package_path }
	}
}

impl Url {
	pub async fn for_path(path: &Path) -> Result<Url> {
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
		let module_path = path
			.strip_prefix(&package_path)
			.unwrap()
			.to_owned()
			.try_into()?;
		Ok(Url::new_path_module(package_path, module_path))
	}
}

impl TryFrom<url::Url> for Url {
	type Error = anyhow::Error;

	fn try_from(value: url::Url) -> Result<Self, Self::Error> {
		match value.scheme() {
			TANGRAM_BUILTINS_SCHEME => {
				let path = value.path().into();
				Ok(Url::Builtins { path })
			},

			TANGRAM_LIB_SCHEME => {
				let path = value.path().into();
				Ok(Url::Lib { path })
			},

			TANGRAM_PACKAGE_MODULE_SCHEME => {
				let package_hash = value
					.domain()
					.with_context(|| format!(r#"The URL "{value}" is missing a domain."#))?
					.parse()
					.context("Failed to parse the domain as a hash.")?;
				let module_path = value
					.query()
					.with_context(|| format!(r#"The URL "{value}" must have a query."#))?
					.into();
				Ok(Url::PackageModule {
					package_hash,
					module_path,
				})
			},

			TANGRAM_PACKAGE_TARGETS_SCHEME => {
				let package_hash = value
					.domain()
					.context("The URL must have a domain.")?
					.parse()
					.context("Failed to parse the domain as a hash.")?;
				Ok(Url::PackageTargets { package_hash })
			},

			TANGRAM_PATH_MODULE_SCHEME => {
				let package_path = value.path().into();
				let module_path = value
					.query()
					.with_context(|| format!(r#"The URL "{value}" must have a query."#))?
					.into();
				Ok(Url::PathModule {
					package_path,
					module_path,
				})
			},

			TANGRAM_PATH_TARGETS_SCHEME => {
				let package_path = value.path().into();
				Ok(Url::PathTargets { package_path })
			},

			_ => bail!(r#"Invalid URL "{value}"."#),
		}
	}
}

impl From<Url> for url::Url {
	fn from(value: Url) -> Self {
		let url = match value {
			Url::Builtins { path } => {
				format!("{TANGRAM_BUILTINS_SCHEME}://{path}")
			},
			Url::Lib { path } => {
				format!("{TANGRAM_LIB_SCHEME}://{path}")
			},
			Url::PackageModule {
				package_hash,
				module_path,
			} => {
				format!("{TANGRAM_PACKAGE_MODULE_SCHEME}://{package_hash}?{module_path}")
			},
			Url::PackageTargets { package_hash } => {
				format!("{TANGRAM_PACKAGE_TARGETS_SCHEME}://{package_hash}")
			},
			Url::PathModule {
				package_path,
				module_path,
			} => {
				let package_path = package_path.display();
				format!("{TANGRAM_PATH_MODULE_SCHEME}://{package_path}?{module_path}")
			},
			Url::PathTargets { package_path } => {
				let package_path = package_path.display();
				format!("{TANGRAM_PATH_TARGETS_SCHEME}://{package_path}")
			},
		};
		url.parse().unwrap()
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
