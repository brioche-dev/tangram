use super::{
	js,
	url::{Referrer, TANGRAM_SCHEME},
	Compiler,
};
use crate::{
	hash::Hash,
	lockfile::Lockfile,
	manifest::{self, Manifest},
	util::normalize,
};
use anyhow::{bail, Context, Result};
use camino::Utf8Path;
use std::path::Path;
use url::Url;

impl Compiler {
	pub async fn resolve(&self, specifier: &str, referrer: Option<&js::Url>) -> Result<js::Url> {
		// If the specifier starts with /, ./, or ../, then resolve it as a path specifier. Otherwise, resolve it as a URL.
		let url = if specifier.starts_with('/')
			|| specifier.starts_with("./")
			|| specifier.starts_with("../")
		{
			resolve_path(specifier, referrer)?
		} else {
			// Parse the specifier as URL.
			let specifier: Url = specifier
				.parse()
				.with_context(|| format!(r#"The specifier "{specifier}" is not a valid URL."#))?;

			// Handle each supported scheme.
			match specifier.scheme() {
				TANGRAM_SCHEME => self.resolve_tangram(&specifier, referrer).await?,
				_ => specifier.try_into()?,
			}
		};
		Ok(url)
	}
}

fn resolve_path(specifier: &str, referrer: Option<&js::Url>) -> Result<js::Url> {
	// Ensure there is a referrer.
	let referrer = referrer.with_context(|| {
		format!(r#"A specifier with the scheme "{TANGRAM_SCHEME}" must have a referrer."#)
	})?;

	let specifier = Utf8Path::new(specifier);

	// Resolve.
	let url = match referrer {
		js::Url::Lib(js::compiler::url::Lib { path }) => js::Url::Lib(js::compiler::url::Lib {
			path: normalize(&path.join("..").join(specifier)),
		}),

		js::Url::HashModule(js::compiler::url::HashModule {
			package_hash,
			module_path,
		}) => js::Url::HashModule(js::compiler::url::HashModule {
			package_hash: *package_hash,
			module_path: normalize(&module_path.join("..").join(specifier)),
		}),

		js::Url::PathModule(js::compiler::url::PathModule {
			package_path,
			module_path,
		}) => js::Url::PathModule(js::compiler::url::PathModule {
			package_path: package_path.clone(),
			module_path: normalize(&module_path.join("..").join(specifier)),
		}),

		_ => {
			bail!(r#"Cannot resolve specifier "{specifier}" from referrer "{referrer}"."#);
		},
	};

	Ok(url)
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
			js::Url::HashModule(js::compiler::url::HashModule { package_hash, .. })
			| js::Url::HashImport(js::compiler::url::HashImport { package_hash, .. })
			| js::Url::HashTarget(js::compiler::url::HashTarget { package_hash, .. }) => {
				self.resolve_tangram_from_hash(specifier, *package_hash)
					.await
			},

			js::Url::Lib { .. } => bail!("Invalid referrer."),

			js::Url::PathModule(js::compiler::url::PathModule { package_path, .. })
			| js::Url::PathImport(js::compiler::url::PathImport { package_path, .. })
			| js::Url::PathTarget(js::compiler::url::PathTarget { package_path, .. }) => {
				self.resolve_tangram_from_path(specifier, package_path)
					.await
			},
		}
	}

	async fn resolve_tangram_from_hash(
		&self,
		specifier: &url::Url,
		referrer_package_hash: Hash,
	) -> Result<js::Url> {
		// Get the specifier's package name.
		let specifier_package_name = specifier.path();

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

		// Get the specifier's package hash from the referrer's dependencies.
		let specifier_package_hash = referrer_dependencies.get(specifier_package_name).context(
			"Expected the referrer's package dependencies to contain the specifier's package name.",
		)?;

		// Create the URL.
		let url = js::Url::new_hash_import(
			*specifier_package_hash,
			Referrer::Hash(referrer_package_hash),
		);

		Ok(url)
	}

	async fn resolve_tangram_from_path(
		&self,
		specifier: &url::Url,
		referrer_package_path: &Path,
	) -> Result<js::Url> {
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

				// Create the URL.
				let url = js::Url::new_path_import(
					specifier_package_path,
					Referrer::Path(referrer_package_path.to_owned()),
				);

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

				// Create the URL.
				let url = js::Url::new_hash_import(
					specifier_hash,
					Referrer::Path(referrer_package_path.to_owned()),
				);

				Ok(url)
			},
		}
	}
}
