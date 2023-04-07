use super::{Data, Hash, Instance};
use crate::{
	error::Result,
	package::{dependency, Package},
};
use std::collections::BTreeMap;

impl Instance {
	pub async fn new(
		tg: &crate::instance::Instance,
		package: Package,
		dependencies: BTreeMap<dependency::Specifier, Instance>,
	) -> Result<Instance> {
		// Get the dependencies' package instance hashes.
		let dependencies: BTreeMap<dependency::Specifier, Hash> = dependencies
			.into_iter()
			.map(|(specifier, package_instance)| (specifier, package_instance.hash()))
			.collect();

		// Create the package instance data.
		let data = Data {
			package_artifact_hash: package.artifact().hash(),
			dependencies: dependencies.clone(),
		};

		// Serialize and hash the package instance data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = Hash(crate::hash::Hash::new(&bytes));

		// Add the package instance data.
		let hash = tg.database.add_package_instance(hash, &bytes).await?;

		// Create the package instance.
		let package_instance = Self {
			hash,
			package,
			dependencies,
		};

		Ok(package_instance)
	}
}
