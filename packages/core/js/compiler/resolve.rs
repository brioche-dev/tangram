use super::{js, Compiler};
use crate::{
	hash::Hash,
	lockfile::Lockfile,
	manifest::{self, Manifest},
};
use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::path::Path;

impl Compiler {
	pub async fn resolve(&self, specifier: &str, referrer: Option<&js::Url>) -> Result<js::Url> {
		// Resolve the specifier relative to the referrer.
		let specifier = deno_core::resolve_import(
			specifier,
			&referrer.map_or_else(|| ".".to_owned(), std::string::ToString::to_string),
		)?;

		let url = match specifier.scheme() {
			"tangram" => self.resolve_tangram(&specifier, referrer).await?,
			_ => specifier.try_into()?,
		};

		Ok(url)
	}

	async fn resolve_tangram(
		&self,
		specifier: &url::Url,
		referrer: Option<&js::Url>,
	) -> Result<js::Url> {
		// Ensure there is a referrer.
		let referrer =
			referrer.context(r#"A specifier with the scheme "tangram" must have a referrer."#)?;

		match referrer {
			js::Url::PackageModule { package_hash, .. } => {
				self.resolve_tangram_from_package(specifier, *package_hash)
					.await
			},
			js::Url::PathModule { package_path, .. } => {
				self.resolve_tangram_from_path(specifier, package_path)
					.await
			},
			_ => bail!("The referrer must have the package module or path module scheme."),
		}
	}

	async fn resolve_tangram_from_package(
		&self,
		specifier: &url::Url,
		referrer_package_hash: Hash,
	) -> Result<js::Url> {
		// Get the referrer's dependencies.
		let referrer_dependencies = self
			.state
			.builder
			.lock_shared()
			.await?
			.get_expression_local(referrer_package_hash)?
			.into_package()
			.context("Expected a package expression.")?
			.dependencies;

		// Get the specifier's package name and sub path.
		let specifier_path = Utf8Path::new(specifier.path());
		let specifier_package_name = specifier_path.components().next().unwrap().as_str();
		let specifier_sub_path = if specifier_path.components().count() > 1 {
			Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
		} else {
			None
		};

		// Get the specifier's package hash from the referrer's dependencies.
		let specifier_package_hash = referrer_dependencies.get(specifier_package_name).context(
			"Expected the referrer's package dependencies to contain the specifier's package name.",
		)?;

		// Compute the URL.
		let url = if let Some(specifier_sub_path) = specifier_sub_path {
			js::Url::new_package_module(*specifier_package_hash, specifier_sub_path)
		} else {
			js::Url::new_package_targets(*specifier_package_hash)
		};

		Ok(url)
	}

	async fn resolve_tangram_from_path(
		&self,
		specifier: &url::Url,
		referrer_package_path: &Path,
	) -> Result<js::Url> {
		// Read the referrer's manifest.
		let referrer_manifest_path = referrer_package_path.join("tangram.json");
		let referrer_manifest = tokio::fs::read(referrer_manifest_path).await?;
		let referrer_manifest: Manifest = serde_json::from_slice(&referrer_manifest)?;

		// Get the specifier's package name and sub path.
		let specifier_path = Utf8Path::new(specifier.path());
		let specifier_package_name = specifier_path.components().next().unwrap().as_str();
		let specifier_sub_path = if specifier_path.components().count() > 1 {
			Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
		} else {
			None
		};

		// Get the specifier's entry in the referrer's manifest.
		let dependency = referrer_manifest
			.dependencies
			.as_ref()
			.and_then(|dependencies| dependencies.get(specifier_package_name))
			.context("Failed to find the specifier in the referrer's lockfile.")?;

		match dependency {
			manifest::Dependency::PathDependency(dependency) => {
				// Compute the URL.
				let specifier_package_path = referrer_package_path.join(&dependency.path);
				let specifier_package_path =
					tokio::fs::canonicalize(&specifier_package_path).await?;
				let url = if let Some(specifier_sub_path) = specifier_sub_path {
					js::Url::new_path_module(specifier_package_path, specifier_sub_path)
				} else {
					js::Url::new_path_targets(specifier_package_path)
				};

				Ok(url)
			},
			manifest::Dependency::RegistryDependency(_) => {
				// Read the lockfile.
				let referrer_lockfile_path = referrer_package_path.join("tangram.lock");
				let referrer_lockfile = tokio::fs::read(&referrer_lockfile_path)
					.await
					.context("Failed to read the lockfile.")?;
				let referrer_lockfile: Lockfile = serde_json::from_slice(&referrer_lockfile)
					.context("Failed to deserialize the lockfile.")?;

				// Get the specifier's entry in the referrer's lockfile.
				let dependency = referrer_lockfile
					.as_v1()
					.unwrap()
					.dependencies
					.get(specifier_package_name)
					.context("Failed to find the specifier in the referrer's lockfile.")?;
				let specifier_hash = dependency.hash;

				// Compute the URL.
				let url = if let Some(specifier_sub_path) = specifier_sub_path {
					js::Url::new_package_module(specifier_hash, specifier_sub_path)
				} else {
					js::Url::new_package_targets(specifier_hash)
				};

				Ok(url)
			},
		}
	}
}
