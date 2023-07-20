pub use self::{dependency::Dependency, metadata::Metadata, specifier::Specifier};
use crate::{
	artifact::Artifact,
	block::Block,
	error::{Result, WrapErr},
	instance::Instance,
	module::{self, Module},
};
use std::collections::BTreeMap;

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
	dependencies: Option<BTreeMap<Dependency, Block>>,
}

impl Package {
	pub async fn with_specifier(tg: &Instance, specifier: Specifier) -> Result<Self> {
		match specifier {
			Specifier::Path(path) => Ok(Self::with_path(tg, &path).await?),
			Specifier::Registry(_) => unimplemented!(),
		}
	}

	#[must_use]
	pub fn artifact(&self) -> &Artifact {
		&self.artifact
	}

	#[must_use]
	pub fn dependencies(&self) -> &Option<BTreeMap<Dependency, Block>> {
		&self.dependencies
	}

	#[must_use]
	pub fn root_module(&self) -> Module {
		Module::Normal(module::Normal {
			package: self.artifact.block(),
			module_path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		})
	}

	pub async fn unlock(&self, tg: &Instance) -> Result<Package> {
		let directory = self
			.artifact
			.as_directory()
			.wrap_err("Expected a directory.")?;
		let builder = directory.builder(tg).await?;
		let builder = builder
			.remove(tg, &LOCKFILE_FILE_NAME.parse().unwrap())
			.await?;
		let artifact = builder.build(tg)?.into();
		Ok(Package {
			artifact,
			dependencies: None,
		})
	}
}

impl std::hash::Hash for Package {
	fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
		self.artifact.hash(state);
	}
}
