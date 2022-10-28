use super::{js, Compiler};
use crate::{
	hash::Hash,
	lockfile::Lockfile,
	manifest::{self, Manifest},
};
use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::path::Path;
use url::Url;

impl Compiler {
	pub async fn resolve(&self, specifier: &str, referrer: Option<&js::Url>) -> Result<js::Url> {
		// If the specifier starts with /, ./, or ../, then resolve it as a relative path. Otherwise, resolve it as an absolute URL.
		let url = if specifier.starts_with('/')
			|| specifier.starts_with("./")
			|| specifier.starts_with("../")
		{
			resolve_relative(specifier, referrer)?
		} else {
			// Parse the specifier as URL.
			let specifier: Url = specifier
				.parse()
				.with_context(|| format!(r#"The specifier "{specifier}" is not a valid URL."#))?;

			// Handle each supported scheme.
			match specifier.scheme() {
				"tangram" => self.resolve_tangram(&specifier, referrer).await?,
				_ => specifier.try_into()?,
			}
		};
		Ok(url)
	}
}

fn resolve_relative(specifier: &str, referrer: Option<&js::Url>) -> Result<js::Url> {
	// Ensure there is a referrer.
	let referrer =
		referrer.context(r#"A specifier with the scheme "tangram" must have a referrer."#)?;

	let specifier = Utf8Path::new(specifier);

	// Resolve.
	let url = match referrer {
		js::Url::PackageModule {
			package_hash,
			module_path,
		} => js::Url::PackageModule {
			package_hash: *package_hash,
			module_path: resolve_path(module_path, specifier)?,
		},
		js::Url::PathModule {
			package_path,
			module_path,
		} => js::Url::PathModule {
			package_path: package_path.clone(),
			module_path: resolve_path(module_path, specifier)?,
		},
		js::Url::TsLib { path } => js::Url::TsLib {
			path: resolve_path(path, specifier)?,
		},
		_ => {
			bail!(r#"Cannot resolve specifier "{specifier}" from referrer "{referrer}"."#);
		},
	};

	Ok(url)
}

fn resolve_path(referrer: &Utf8Path, specifier: &Utf8Path) -> Result<Utf8PathBuf> {
	let mut path = Utf8PathBuf::new();
	for component in referrer
		.parent()
		.unwrap_or(referrer)
		.join(specifier)
		.components()
	{
		match component {
			camino::Utf8Component::Prefix(prefix) => {
				bail!(r#"Invalid path component "{prefix}"."#);
			},
			camino::Utf8Component::RootDir => {
				path = Utf8PathBuf::from("/");
			},
			camino::Utf8Component::CurDir => {},
			camino::Utf8Component::ParentDir => {
				let popped = path.pop();
				if !popped {
					bail!(r#"Specifier "{specifier}" escapes path "{referrer}"."#);
				}
			},
			camino::Utf8Component::Normal(string) => {
				path.push(string);
			},
		}
	}
	Ok(path)
}

impl Compiler {
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
		let specifier_module_path = if specifier_path.components().count() > 1 {
			Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
		} else {
			None
		};

		// Get the specifier's package hash from the referrer's dependencies.
		let specifier_package_hash = referrer_dependencies.get(specifier_package_name).context(
			"Expected the referrer's package dependencies to contain the specifier's package name.",
		)?;

		// Compute the URL.
		let url = if let Some(specifier_module_path) = specifier_module_path {
			js::Url::new_package_module(*specifier_package_hash, specifier_module_path)
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
		let specifier_module_path = if specifier_path.components().count() > 1 {
			Some(specifier_path.components().skip(1).collect::<Utf8PathBuf>())
		} else {
			None
		};

		// Get the specifier's entry in the referrer's manifest.
		let dependency = referrer_manifest
			.dependencies
			.as_ref()
			.and_then(|dependencies| dependencies.get(specifier_package_name))
			.with_context(|| format!("Failed to find the specifier {specifier_package_name:?} in the referrer's lockfile at {}.", referrer_package_path.display()))?;

		match dependency {
			manifest::Dependency::PathDependency(dependency) => {
				// Compute the URL.
				let specifier_package_path = referrer_package_path.join(&dependency.path);
				let specifier_package_path =
					tokio::fs::canonicalize(&specifier_package_path).await?;
				let url = if let Some(specifier_module_path) = specifier_module_path {
					js::Url::new_path_module(specifier_package_path, specifier_module_path)
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
					.with_context(|| format!("Failed to find the specifier {specifier_package_name:?} in the referrer's lockfile at {} for a registry dependency.", referrer_lockfile_path.display()))?;
				let specifier_hash = dependency.hash;

				// Compute the URL.
				let url = if let Some(specifier_module_path) = specifier_module_path {
					js::Url::new_package_module(specifier_hash, specifier_module_path)
				} else {
					js::Url::new_package_targets(specifier_hash)
				};

				Ok(url)
			},
		}
	}
}
