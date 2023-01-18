use super::{module_identifier::TANGRAM_SCHEME, ModuleIdentifier};
use crate::Cli;
use crate::{
	lockfile::Lockfile,
	manifest::{self, Manifest},
	package::PackageHash,
	util::normalize,
};
use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use std::path::Path;
use url::Url;

impl Cli {
	pub async fn resolve(
		&self,
		specifier: &str,
		referrer: Option<&ModuleIdentifier>,
	) -> Result<ModuleIdentifier> {
		let module_identifier = if specifier.starts_with('/')
			|| specifier.starts_with("./")
			|| specifier.starts_with("../")
		{
			// If the specifier starts with /, ./, or ../, then resolve it as a path specifier.
			Self::resolve_path(specifier, referrer)?
		} else if specifier == "tangram" {
			// If the specifier is "tangram", then resolve it to the root core module.
			ModuleIdentifier::new_core("/mod.ts".into())
		} else {
			// Parse the specifier as a URL.
			let specifier: Url = specifier
				.parse()
				.with_context(|| format!(r#"The specifier "{specifier}" is not a valid URL."#))?;

			// Handle each supported scheme.
			match specifier.scheme() {
				TANGRAM_SCHEME => self.resolve_tangram(&specifier, referrer).await?,
				_ => specifier.try_into()?,
			}
		};
		Ok(module_identifier)
	}
}

impl Cli {
	fn resolve_path(
		specifier: &str,
		referrer: Option<&ModuleIdentifier>,
	) -> Result<ModuleIdentifier> {
		// Ensure there is a referrer.
		let referrer = referrer.with_context(|| {
			format!(r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer."#)
		})?;

		let specifier = Utf8Path::new(specifier);

		// Resolve.
		let module_identifier = match referrer {
			ModuleIdentifier::Lib { path } => ModuleIdentifier::Lib {
				path: normalize(&path.join("..").join(specifier)),
			},

			ModuleIdentifier::Core { path } => ModuleIdentifier::Core {
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
		specifier: &Url,
		referrer: Option<&ModuleIdentifier>,
	) -> Result<ModuleIdentifier> {
		// Ensure there is a referrer.
		let referrer =
			referrer.context(r#"A specifier with the scheme "tangram" must have a referrer."#)?;

		match referrer {
			ModuleIdentifier::Lib { .. } | ModuleIdentifier::Core { .. } => {
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
		specifier: &Url,
		referrer_package_hash: PackageHash,
	) -> Result<ModuleIdentifier> {
		// Get the specifier's package name.
		let specifier_package_name = specifier.path();

		// Get the referrer's dependencies.
		let referrer_dependencies = self.get_package_local(referrer_package_hash)?.dependencies;

		// Get the specifier's package hash from the referrer's dependencies.
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
		specifier: &Url,
		referrer_package_path: &Path,
	) -> Result<ModuleIdentifier> {
		// Get the specifier's package name.
		let specifier_package_name = specifier.path();

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
