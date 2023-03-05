pub use self::{identifier::Identifier, instance::Instance, specifier::Specifier};
use anyhow::Result;
use async_recursion::async_recursion;
use std::{collections::BTreeMap, sync::Arc};

pub mod checkin;
pub mod dependency;
pub mod identifier;
pub mod instance;
mod lockfile;
mod resolve;
pub mod specifier;

impl crate::Instance {
	#[allow(clippy::unused_async, clippy::only_used_in_recursion)]
	#[async_recursion]
	pub async fn create_package_instance(
		self: &Arc<Self>,
		package_identifier: &Identifier,
		locked: bool,
	) -> Result<instance::Hash> {
		// Get the package hash and dependency specifier.
		let checkin::Output {
			package_hash,
			dependency_specifiers,
		} = match package_identifier {
			Identifier::Path(path) => self.check_in_package(path).await?,

			Identifier::Hash(_) => todo!(),
		};

		// Create the dependencies.
		let mut dependencies = BTreeMap::default();
		for dependency_specifier in dependency_specifiers {
			let dependency_identifier = self
				.resolve_package(
					&dependency_specifier.clone().into(),
					Some(package_identifier),
				)
				.await?;
			let dependency_instance = self
				.create_package_instance(&dependency_identifier, locked)
				.await?;
			dependencies.insert(dependency_specifier, dependency_instance);
		}

		// Create the package instance.
		let package_instance = Instance {
			package_hash,
			dependencies,
		};

		// Add the package instance.
		let package_instance_hash = self.add_package_instance(&package_instance)?;

		Ok(package_instance_hash)
	}
}
