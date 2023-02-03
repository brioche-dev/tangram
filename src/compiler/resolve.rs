use super::{module_specifier::ModuleSpecifier, ModuleIdentifier};
use crate::{
	lockfile::Lockfile,
	manifest::{self, Manifest},
	package::PackageHash,
	package_specifier::PackageSpecifier,
	util::normalize,
	Cli,
};
use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use std::path::Path;

impl Cli {
	pub async fn resolve(
		&self,
		specifier: &ModuleSpecifier,
		referrer: &ModuleIdentifier,
	) -> Result<ModuleIdentifier> {
		let module_identifier = match specifier {
			ModuleSpecifier::Path { module_path } => Self::resolve_path(&module_path, referrer)?,
			ModuleSpecifier::Package(package_specifier) => {
				self.resolve_tangram(&package_specifier, referrer).await?
			},
		};

		Ok(module_identifier)
	}
}

impl Cli {
	pub fn resolve_path(
		specifier: &Utf8Path,
		referrer: &ModuleIdentifier,
	) -> Result<ModuleIdentifier> {
		// Resolve.
		let module_identifier = match referrer {
			ModuleIdentifier::Lib { path } => ModuleIdentifier::Lib {
				path: normalize(&path.join("..").join(specifier)),
			},

			ModuleIdentifier::Hash {
				package_hash,
				module_path,
			} => ModuleIdentifier::Hash {
				package_hash: *package_hash,
				module_path: normalize(&module_path.join("..").join(specifier)),
			},

			ModuleIdentifier::Path {
				package_path,
				module_path,
			} => ModuleIdentifier::Path {
				package_path: package_path.clone(),
				module_path: normalize(&module_path.join("..").join(specifier)),
			},
		};

		Ok(module_identifier)
	}
}

impl Cli {
	async fn resolve_tangram(
		&self,
		specifier: &PackageSpecifier,
		referrer: &ModuleIdentifier,
	) -> Result<ModuleIdentifier> {
		match referrer {
			ModuleIdentifier::Lib { .. } => {
				bail!("Invalid referrer.")
			},

			ModuleIdentifier::Hash { package_hash, .. } => {
				self.resolve_tangram_from_hash(specifier, *package_hash)
			},

			ModuleIdentifier::Path { package_path, .. } => {
				self.resolve_tangram_from_path(specifier, package_path)
					.await
			},
		}
	}

	fn resolve_tangram_from_hash(
		&self,
		specifier: &PackageSpecifier,
		referrer_package_hash: PackageHash,
	) -> Result<ModuleIdentifier> {
		// Get the specifier's package name.
		let specifier_package_name = specifier.key();

		// Get the referrer's dependencies.
		let referrer_dependencies = self.get_package_local(referrer_package_hash)?.dependencies;

		// Get the specifier's package hash from the referrer's dependencies.
		// TODO: Support more sophisticated resolution.
		let specifier_package_hash = referrer_dependencies.get(specifier_package_name).context(
			"Expected the referrer's package dependencies to contain the specifier's package name.",
		)?;

		// Create the module identifier.
		let module_identifier =
			ModuleIdentifier::new_hash(*specifier_package_hash, "package.tg".into());

		Ok(module_identifier)
	}

	async fn resolve_tangram_from_path(
		&self,
		specifier: &PackageSpecifier,
		referrer_package_path: &Path,
	) -> Result<ModuleIdentifier> {
		// Get the specifier's package name.
		let specifier_package_name = specifier.key();

		// Read the referrer's manifest.
		let referrer_manifest_path = referrer_package_path.join("tangram.json");
		let referrer_manifest = tokio::fs::read(referrer_manifest_path).await?;
		let referrer_manifest: Manifest = serde_json::from_slice(&referrer_manifest)?;

		// Get the specifier's entry in the referrer's manifest.
		let dependency = referrer_manifest
			.dependencies
			.as_ref()
			.and_then(|dependencies| dependencies.get(specifier_package_name))
			.with_context(|| format!("Failed to find the specifier {specifier_package_name:?} in the referrer's lockfile at {}.", referrer_package_path.display()))?;

		match dependency {
			manifest::Dependency::PathDependency(dependency) => {
				// Compute the specifier package path.
				let specifier_package_path = referrer_package_path.join(&dependency.path);
				let specifier_package_path =
					tokio::fs::canonicalize(&specifier_package_path).await?;

				// Create the module identifier.
				let module_identifier =
					ModuleIdentifier::new_path(specifier_package_path, "package.tg".into());

				Ok(module_identifier)
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
					.with_context(|| format!("Failed to find the specifier {specifier_package_name:?} in the referrer's lockfile at {} for a registry dependency.", referrer_lockfile_path.display()))?;
				let specifier_hash = dependency.hash;

				// Create the module identifier.
				let module_identifier =
					ModuleIdentifier::new_hash(specifier_hash, "package.tg".into());

				Ok(module_identifier)
			},
		}
	}
}
