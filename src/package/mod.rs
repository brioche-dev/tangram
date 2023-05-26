pub use self::{dependency::Dependency, metadata::Metadata, specifier::Specifier};
pub use crate::artifact::Hash;
use crate::{
	artifact::Artifact,
	error::Result,
	module::{self, Module},
};
use std::{collections::BTreeMap, sync::Arc};

/// The file name of the root module in a package.
pub const ROOT_MODULE_FILE_NAME: &str = "tangram.tg";

/// The file name of the lockfile.
pub const LOCKFILE_FILE_NAME: &str = "tangram.lock";

pub mod dependency;
mod get;
pub mod lockfile;
pub mod metadata;
mod path;
pub mod specifier;

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Package {
	artifact: Artifact,
	dependencies: Option<BTreeMap<Dependency, Hash>>,
}

impl Package {
	pub async fn from_specifier(
		tg: &Arc<crate::instance::Instance>,
		specifier: Specifier,
	) -> Result<Self> {
		match specifier {
			Specifier::Path(path) => Ok(Self::from_path(tg, &path).await?),
			Specifier::Registry(_) => unimplemented!(),
		}
	}

	#[must_use]
	pub fn hash(&self) -> Hash {
		self.artifact.hash()
	}

	#[must_use]
	pub fn artifact(&self) -> &Artifact {
		&self.artifact
	}

	#[must_use]
	pub fn dependencies(&self) -> &Option<BTreeMap<Dependency, Hash>> {
		&self.dependencies
	}

	#[must_use]
	pub fn root_module(&self) -> Module {
		Module::Normal(module::Normal {
			package_hash: self.hash(),
			module_path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		})
	}
}

impl std::hash::Hash for Package {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		std::hash::Hash::hash(&self.artifact, state);
	}
}
