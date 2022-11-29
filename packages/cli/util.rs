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
		let mut arg = BTreeMap::new();
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
		arg.insert("target".into(), system);
		let arg = builder
			.add_expression(&tangram_core::expression::Expression::Map(arg))
			.await?;
		let args = vec![arg];
		let args = builder
			.add_expression(&tangram_core::expression::Expression::Array(args))
			.await?;
		Ok(args)
	}
}

impl Cli {
	pub async fn js_urls_for_specifier(&self, specifier: &Specifier) -> Result<Vec<js::Url>> {
		match &specifier {
			Specifier::Package(package_specifier) => {
				let package_hash = self
					.get_package_hash_from_specifier(package_specifier)
					.await?;
				let url = js::Url::new_hash_module(package_hash, "tangram.ts".into());
				Ok(vec![url])
			},

			Specifier::Path(path) => {
				let path = std::env::current_dir()
					.context("Failed to get the current directory")?
					.join(path);
				let path = tokio::fs::canonicalize(&path).await?;
				let url = js::Url::new_path_module(path, "tangram.ts".into());
				Ok(vec![url])
			},
		}
	}

	pub async fn package_hash_for_specifier(
		&self,
		specifier: &Specifier,
		locked: bool,
	) -> Result<Hash> {
		match specifier {
			Specifier::Package(package_specifier) => {
				let package_hash = self
					.get_package_hash_from_specifier(package_specifier)
					.await?;
				Ok(package_hash)
			},

			Specifier::Path(path) => {
				let package_hash = self
					.builder
					.lock_shared()
					.await?
					.checkin_package(&self.api_client, path, locked)
					.await
					.with_context(|| {
						format!("Failed to create the package for specifier '{specifier}'.")
					})?;
				Ok(package_hash)
			},
		}
	}

	pub async fn get_package_hash_from_specifier(
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
