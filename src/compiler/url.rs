use crate::{hash::Hash, util::path_exists};
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
	HashModule(HashModule),
	HashImport(HashImport),
	HashTarget(HashTarget),
	Lib(Lib),
	PathModule(PathModule),
	PathImport(PathImport),
	PathTarget(PathTarget),
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct HashModule {
	pub package_hash: Hash,
	pub module_path: Utf8PathBuf,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct HashTarget {
	pub package_hash: Hash,
	pub module_path: Utf8PathBuf,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct HashImport {
	pub package_hash: Hash,
	pub referrer: Referrer,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Lib {
	pub path: Utf8PathBuf,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct PathModule {
	pub package_path: PathBuf,
	pub module_path: Utf8PathBuf,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct PathTarget {
	pub package_path: PathBuf,
	pub module_path: Utf8PathBuf,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct PathImport {
	pub package_path: PathBuf,
	pub referrer: Referrer,
}

#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum Referrer {
	Hash(Hash),
	Path(PathBuf),
}

impl Url {
	#[must_use]
	pub fn new_hash_module(package_hash: Hash, module_path: Utf8PathBuf) -> Url {
		Url::HashModule(HashModule {
			package_hash,
			module_path,
		})
	}

	#[must_use]
	pub fn new_hash_import(package_hash: Hash, referrer: Referrer) -> Url {
		Url::HashImport(HashImport {
			package_hash,
			referrer,
		})
	}

	#[must_use]
	pub fn new_hash_target(package_hash: Hash, module_path: Utf8PathBuf) -> Url {
		Url::HashTarget(HashTarget {
			package_hash,
			module_path,
		})
	}

	#[must_use]
	pub fn new_path_module(package_path: PathBuf, module_path: Utf8PathBuf) -> Url {
		Url::PathModule(PathModule {
			package_path,
			module_path,
		})
	}

	#[must_use]
	pub fn new_path_import(package_path: PathBuf, referrer: Referrer) -> Url {
		Url::PathImport(PathImport {
			package_path,
			referrer,
		})
	}

	#[must_use]
	pub fn new_path_target(package_path: PathBuf, module_path: Utf8PathBuf) -> Url {
		Url::PathTarget(PathTarget {
			package_path,
			module_path,
		})
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
		let url = Url::new_path_module(package_path, module_path);
		Ok(url)
	}
}

impl TryFrom<url::Url> for Url {
	type Error = anyhow::Error;

	fn try_from(value: url::Url) -> Result<Self, Self::Error> {
		// Validate the scheme.
		let scheme = value.scheme();
		if scheme != TANGRAM_INTERNAL_SCHEME {
			bail!(r#"Invalid scheme "{scheme}"."#);
		}

		// Get the kind.
		let kind = value.domain().context("A domain is required.")?;

		let url = match kind {
			"hash_module" => {
				let data = value.query().context("The URL must have a query.")?;
				let data = hex::decode(data)?;
				let data = serde_json::from_slice(&data)?;
				Url::HashModule(data)
			},

			"hash_import" => {
				let data = value.query().context("The URL must have a query.")?;
				let data = hex::decode(data)?;
				let data = serde_json::from_slice(&data)?;
				Url::HashImport(data)
			},

			"hash_target" => {
				let data = value.query().context("The URL must have a query.")?;
				let data = hex::decode(data)?;
				let data = serde_json::from_slice(&data)?;
				Url::HashTarget(data)
			},

			"lib" => {
				let path = value.path().into();
				Url::Lib(Lib { path })
			},

			"path_module" => {
				let data = value.query().context("The URL must have a query.")?;
				let data = hex::decode(data)?;
				let data = serde_json::from_slice(&data)?;
				Url::PathModule(data)
			},

			"path_import" => {
				let data = value.query().context("The URL must have a query.")?;
				let data = hex::decode(data)?;
				let data = serde_json::from_slice(&data)?;
				Url::PathImport(data)
			},

			"path_target" => {
				let data = value.query().context("The URL must have a query.")?;
				let data = hex::decode(data)?;
				let data = serde_json::from_slice(&data)?;
				Url::PathTarget(data)
			},

			_ => bail!(r#"Invalid URL "{value}"."#),
		};

		Ok(url)
	}
}

impl From<Url> for url::Url {
	fn from(value: Url) -> Self {
		match value {
			Url::HashModule(value) => {
				let data = serde_json::to_vec(&value).unwrap();
				let data = hex::encode(&data);
				format!("{TANGRAM_INTERNAL_SCHEME}://hash_module/?{data}#.ts")
			},

			Url::HashImport(value) => {
				let data = serde_json::to_vec(&value).unwrap();
				let data = hex::encode(&data);
				format!("{TANGRAM_INTERNAL_SCHEME}://hash_import/?{data}#.ts")
			},

			Url::HashTarget(value) => {
				let data = serde_json::to_vec(&value).unwrap();
				let data = hex::encode(&data);
				format!("{TANGRAM_INTERNAL_SCHEME}://hash_target/?{data}#.ts")
			},

			Url::Lib(value) => {
				let path = value.path;
				format!("{TANGRAM_INTERNAL_SCHEME}://lib{path}")
			},

			Url::PathModule(value) => {
				let data = serde_json::to_vec(&value).unwrap();
				let data = hex::encode(&data);
				format!("{TANGRAM_INTERNAL_SCHEME}://path_module/?{data}#.ts")
			},

			Url::PathImport(value) => {
				let data = serde_json::to_vec(&value).unwrap();
				let data = hex::encode(&data);
				format!("{TANGRAM_INTERNAL_SCHEME}://path_import/?{data}#.ts")
			},

			Url::PathTarget(value) => {
				let data = serde_json::to_vec(&value).unwrap();
				let data = hex::encode(&data);
				format!("{TANGRAM_INTERNAL_SCHEME}://path_target/?{data}#.ts")
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
