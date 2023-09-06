pub use self::{dependency::Dependency, metadata::Metadata, specifier::Specifier};
use crate::{artifact, error::Result, Artifact, Package};
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

crate::id!();

crate::kind!(Package);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub struct Value {
	pub artifact: Artifact,
	pub dependencies: Option<BTreeMap<Dependency, Package>>,
}

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub artifact: crate::artifact::Id,

	#[tangram_serialize(id = 1)]
	pub dependencies: Option<BTreeMap<Dependency, crate::package::Id>>,
}

impl Handle {
	// pub async fn with_specifier(tg: &Client, specifier: Specifier) -> Result<Self> {
	// 	match specifier {
	// 		Specifier::Path(path) => Ok(Self::with_path(tg, &path).await?),
	// 		Specifier::Registry(_) => unimplemented!(),
	// 	}
	// }

	// #[must_use]
	// pub fn artifact(&self) -> &Artifact {
	// 	&self.artifact
	// }

	// #[must_use]
	// pub fn dependencies(&self) -> &Option<BTreeMap<Dependency, Package>> {
	// 	&self.dependencies
	// }

	// pub async fn root_module(&self, tg: &Client) -> Result<Module> {
	// 	Ok(Module::Normal(module::Normal {
	// 		package: todo!(),
	// 		path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
	// 	}))
	// }

	// #[must_use]
	// pub fn to_unlocked(&self) -> Value {
	// 	Self {
	// 		artifact: self.artifact.clone(),
	// 		dependencies: None,
	// 	}
	// }
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let artifact = artifact::Handle::with_id(data.artifact);
		let dependencies = data.dependencies.map(|dependencies| {
			dependencies
				.into_iter()
				.map(|(dependency, id)| (dependency, Handle::with_id(id)))
				.collect()
		});
		Self {
			artifact,
			dependencies,
		}
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		let artifact = self.artifact.expect_id();
		let dependencies = self.dependencies.as_ref().map(|dependencies| {
			dependencies
				.iter()
				.map(|(dependency, id)| (dependency.clone(), id.expect_id()))
				.collect()
		});
		Data {
			artifact,
			dependencies,
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		let mut children = vec![];
		children.extend(
			self.dependencies
				.as_ref()
				.map(|dependencies| {
					dependencies
						.values()
						.cloned()
						.map(Into::into)
						.collect::<Vec<_>>()
				})
				.unwrap_or_default(),
		);
		children.push(self.artifact.clone().into());
		children
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		std::iter::once(self.artifact.into())
			.chain(
				self.dependencies
					.iter()
					.flatten()
					.map(|(_, id)| (*id).into()),
			)
			.collect()
	}
}
