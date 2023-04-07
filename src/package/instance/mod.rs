pub use self::{data::Data, hash::Hash};
use super::{dependency, Package, ROOT_MODULE_FILE_NAME};
use crate::{
	error::{Error, Result},
	module::{self, Module},
};
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::collections::BTreeMap;

mod data;
mod get;
mod hash;
mod new;

#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
	/// The package instance's hash.
	hash: Hash,

	/// The package.
	package: Package,

	/// The dependencies.
	dependencies: BTreeMap<dependency::Specifier, Hash>,
}

impl Instance {
	#[must_use]
	pub fn hash(&self) -> Hash {
		self.hash
	}

	#[must_use]
	pub fn package(&self) -> &Package {
		&self.package
	}

	pub async fn dependencies(
		&self,
		tg: &crate::instance::Instance,
	) -> Result<BTreeMap<dependency::Specifier, Instance>> {
		self.dependencies
			.iter()
			.map(|(specifier, hash)| async move {
				let package_instance = Instance::get(tg, *hash).await?;
				Ok::<_, Error>((specifier.clone(), package_instance))
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await
	}

	#[must_use]
	pub fn root_module(&self) -> Module {
		Module::Normal(module::Normal {
			package_instance_hash: self.hash,
			module_path: ROOT_MODULE_FILE_NAME.into(),
		})
	}
}
