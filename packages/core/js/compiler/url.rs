use crate::{hash::Hash, util::path_exists};
use anyhow::{bail, Context, Result};
use camino::Utf8PathBuf;
use std::path::{Path, PathBuf};

pub const TANGRAM_SCHEME: &str = "tangram";

pub const TANGRAM_TS_LIB_SCHEME: &str = "tangram-typescript-lib";

pub const TANGRAM_PACKAGE_MODULE_SCHEME: &str = "tangram-package-module";
pub const TANGRAM_PACKAGE_TARGETS_SCHEME: &str = "tangram-package-targets";
pub const TANGRAM_PATH_MODULE_SCHEME: &str = "tangram-path-module";
pub const TANGRAM_PATH_TARGETS_SCHEME: &str = "tangram-path-targets";

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "url::Url", try_from = "url::Url")]
pub enum Url {
	PackageModule {
		package_hash: Hash,
		sub_path: Utf8PathBuf,
	},
	PackageTargets {
		package_hash: Hash,
	},
	PathModule {
		package_path: PathBuf,
		sub_path: Utf8PathBuf,
	},
	PathTargets {
		package_path: PathBuf,
	},
	TsLib,
}

impl Url {
	#[must_use]
	pub fn new_package_module(package_hash: Hash, sub_path: Utf8PathBuf) -> Url {
		Url::PackageModule {
			package_hash,
			sub_path,
		}
	}

	#[must_use]
	pub fn new_package_targets(package_hash: Hash) -> Url {
		Url::PackageTargets { package_hash }
	}

	#[must_use]
	pub fn new_path_module(path: PathBuf, sub_path: Utf8PathBuf) -> Url {
		Url::PathModule {
			package_path: path,
			sub_path,
		}
	}

	#[must_use]
	pub fn new_path_targets(path: PathBuf) -> Url {
		Url::PathTargets { package_path: path }
	}
}

impl Url {
	pub async fn new_for_module_path(path: &Path) -> Result<Url> {
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
		let sub_path = path
			.strip_prefix(&package_path)
			.unwrap()
			.to_owned()
			.try_into()?;
		Ok(Url::new_path_module(package_path, sub_path))
	}
}

impl Url {
	pub async fn from_typescript_path(path: &str) -> Result<Url> {
		let path = Utf8PathBuf::from(path);

		let mut components = path.components();
		components.next().context("Invalid path.")?;
		let first_component = components.next().context("Invalid path.")?.as_str();

		let url = match first_component {
			"__tangram_package_module__" => {
				let package_hash = components
					.next()
					.context("Invalid path.")?
					.as_str()
					.parse()?;
				let sub_path = components.collect();
				Url::new_package_module(package_hash, sub_path)
			},

			"__tangram_package_targets__" => {
				let package_hash = components
					.next()
					.context("Invalid path.")?
					.as_str()
					.parse()?;
				Url::new_package_targets(package_hash)
			},

			"__tangram_path_module__" => {
				let mut path: Utf8PathBuf = Utf8PathBuf::from("/");
				for component in components {
					path.push(component);
				}
				let path: PathBuf = path.into();
				Url::new_for_module_path(&path).await?
			},

			"__tangram_path_targets__" => {
				let mut path: Utf8PathBuf = Utf8PathBuf::from("/");
				for component in components {
					path.push(component);
				}
				path.pop();
				Url::new_path_targets(path.into())
			},

			"__tangram_typescript_lib__" => Url::TsLib,

			_ => bail!("Invalid path."),
		};

		Ok(url)
	}

	#[must_use]
	pub fn to_typescript_path(&self) -> String {
		match self {
			Url::PackageModule {
				package_hash,
				sub_path,
			} => {
				format!("/__tangram_package_module__/{package_hash}/{sub_path}")
			},

			Url::PackageTargets { package_hash } => {
				format!("/__tangram_package_targets__/{package_hash}/targets.ts")
			},

			Url::PathModule {
				package_path,
				sub_path,
			} => {
				format!(
					"/__tangram_path_module__/{package_path}/{sub_path}",
					package_path = package_path.strip_prefix("/").unwrap().display(),
				)
			},

			Url::PathTargets { package_path } => {
				format!(
					"/__tangram_path_targets__/{package_path}/targets.ts",
					package_path = package_path.strip_prefix("/").unwrap().display(),
				)
			},

			Url::TsLib => "/__tangram_typescript_lib__/lib.d.ts".to_owned(),
		}
	}
}

impl TryFrom<url::Url> for Url {
	type Error = anyhow::Error;

	fn try_from(value: url::Url) -> Result<Self, Self::Error> {
		match value.scheme() {
			TANGRAM_PACKAGE_MODULE_SCHEME => {
				let package_hash = value
					.domain()
					.context("The URL must have a domain.")?
					.parse()
					.context("Failed to parse the domain as a hash.")?;
				let sub_path = value.query().context("The URL must have a query.")?.into();
				Ok(Url::PackageModule {
					package_hash,
					sub_path,
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
				let sub_path = value.query().context("The URL must have a query.")?.into();
				Ok(Url::PathModule {
					package_path,
					sub_path,
				})
			},

			TANGRAM_PATH_TARGETS_SCHEME => {
				let package_path = value.path().into();
				Ok(Url::PathTargets { package_path })
			},

			TANGRAM_TS_LIB_SCHEME => Ok(Url::TsLib),

			_ => bail!(r#"Invalid URL "{value}"."#),
		}
	}
}

impl From<Url> for url::Url {
	fn from(value: Url) -> Self {
		let url = match value {
			Url::PackageModule {
				package_hash,
				sub_path,
			} => {
				format!(
					"{}://{}?{}",
					TANGRAM_PACKAGE_MODULE_SCHEME, package_hash, sub_path
				)
			},
			Url::PackageTargets { package_hash } => {
				format!(
					"{}://{}?targets.ts",
					TANGRAM_PACKAGE_TARGETS_SCHEME, package_hash
				)
			},
			Url::PathModule {
				package_path: path,
				sub_path,
			} => {
				format!(
					"{}://{}?{}",
					TANGRAM_PATH_MODULE_SCHEME,
					path.display(),
					sub_path
				)
			},
			Url::PathTargets { package_path: path } => {
				format!(
					"{}://{}?targets.ts",
					TANGRAM_PATH_TARGETS_SCHEME,
					path.display()
				)
			},
			Url::TsLib => {
				format!("{}:", TANGRAM_TS_LIB_SCHEME)
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
