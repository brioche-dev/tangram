use crate::Cli;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use tangram_core::{
	hash::Hash,
	js,
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
	pub async fn js_url_for_specifier(&self, specifier: &Specifier) -> Result<js::Url> {
		match &specifier {
			Specifier::Path(path) => {
				let path = std::env::current_dir()
					.context("Failed to get the current directory")?
					.join(path);
				let path = tokio::fs::canonicalize(&path).await?;
				let url = js::Url::new_path_targets(path);
				Ok(url)
			},

			Specifier::Package(package_specifier) => {
				let package_hash = self.get_package_version(package_specifier).await?;
				let url = js::Url::new_package_targets(package_hash);
				Ok(url)
			},
		}
	}

	pub async fn package_hash_for_specifier(
		&self,
		specifier: &Specifier,
		locked: bool,
	) -> Result<Hash> {
		match specifier {
			Specifier::Path(path) => {
				let package_hash = self
					.builder
					.lock_shared()
					.await?
					.checkin_package(&self.api_client, path, locked)
					.await
					.context("Failed to create the package.")?;
				Ok(package_hash)
			},

			Specifier::Package(package_specifier) => {
				let package_hash = self.get_package_version(package_specifier).await?;
				Ok(package_hash)
			},
		}
	}

	pub async fn get_package_version(
		&self,
		package_specifier: &specifier::Package,
	) -> Result<Hash> {
		let name = &package_specifier.name;
		let version = package_specifier
			.version
			.as_ref()
			.context("A version is required.")?;
		let hash = self
			.api_client
			.get_package_version(name, version)
			.await
			.with_context(|| {
				format!(r#"Failed to get the package "{name}" at version "{version}"."#)
			})?;
		Ok(hash)
	}
}
