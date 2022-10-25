use crate::Cli;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use tangram_core::{
	hash::Hash,
	specifier::{self, Specifier},
	system::System,
};

impl Cli {
	pub async fn create_target_args(&self, system: Option<System>) -> Result<Hash> {
		let builder = self.builder.lock_shared().await?;
		let mut target_arg = BTreeMap::new();
		let system = if let Some(system) = system {
			system
		} else {
			System::host()?
		};
		let system = builder
			.add_expression(&tangram_core::expression::Expression::String(
				system.to_string().into(),
			))
			.await?;
		target_arg.insert("system".into(), system);
		let target_arg = builder
			.add_expression(&tangram_core::expression::Expression::Map(target_arg))
			.await?;
		let target_args = vec![target_arg];
		let target_args = builder
			.add_expression(&tangram_core::expression::Expression::Array(target_args))
			.await?;
		Ok(target_args)
	}
}

impl Cli {
	pub async fn package_hash_for_specifier(
		&self,
		specifier: &Specifier,
		locked: bool,
	) -> Result<Hash> {
		// Get the package hash.
		let package_hash = match specifier {
			Specifier::Path(path) => {
				// Create the package.
				self.builder
					.lock_shared()
					.await?
					.checkin_package(&self.api_client, path, locked)
					.await
					.context("Failed to create the package.")?
			},

			Specifier::Registry(specifier::Registry {
				package_name,
				version,
			}) => {
				// Get the package from the registry.
				let version = version.as_ref().context("A version is required.")?;
				self.api_client
					.get_package_version(package_name, version)
					.await
					.with_context(|| {
						format!(r#"Failed to get the package "{package_name}" from the registry."#)
					})?
			},
		};
		Ok(package_hash)
	}
}
