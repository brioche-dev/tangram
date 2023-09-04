pub use self::{dependency::Dependency, metadata::Metadata, specifier::Specifier};
use crate::{
	self as tg,
	error::Result,
	module::{self, Module},
	server::Server,
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

crate::value!(Package);

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Package {
	#[tangram_serialize(id = 0)]
	artifact: tg::Artifact,

	#[tangram_serialize(id = 1)]
	dependencies: Option<BTreeMap<Dependency, tg::Package>>,
}

impl Package {
	pub async fn with_specifier(tg: &Server, specifier: Specifier) -> Result<Self> {
		match specifier {
			Specifier::Path(path) => Ok(Self::with_path(tg, &path).await?),
			Specifier::Registry(_) => unimplemented!(),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<tg::Value> {
		let mut children = vec![];
		children.extend(
			self.dependencies
				.as_ref()
				.map(|dependencies| {
					dependencies
						.values()
						.cloned()
						.map(Into::into)
						.collect::<Vec<tg::Value>>()
				})
				.unwrap_or_default(),
		);
		children.push(self.artifact.clone().into());
		children
	}

	#[must_use]
	pub fn artifact(&self) -> &tg::Artifact {
		&self.artifact
	}

	#[must_use]
	pub fn dependencies(&self) -> &Option<BTreeMap<Dependency, tg::Package>> {
		&self.dependencies
	}

	pub async fn root_module(&self, tg: &Server) -> Result<Module> {
		Ok(Module::Normal(module::Normal {
			package: self.artifact.id(tg).await?,
			path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
		}))
	}

	#[must_use]
	pub fn to_unlocked(&self) -> Package {
		Self {
			artifact: self.artifact.clone(),
			dependencies: None,
		}
	}
}
