use super::{
	dependency,
	identifier::{self, Lib, Source},
	Identifier, Specifier,
};
use crate::{os, package, path::Path, Instance};
use anyhow::{bail, Context, Result};

impl Instance {
	/// Resolve a specifier relative to a referrer.
	pub async fn resolve_module(
		&self,
		specifier: &Specifier,
		referrer: &Identifier,
	) -> Result<Identifier> {
		let identifier = match specifier {
			Specifier::Path(path) => {
				Self::resolve_module_with_path_specifier(path, referrer).await?
			},
			Specifier::Dependency(dependency_specifier) => {
				self.resolve_module_with_dependency_specifier(dependency_specifier, referrer)
					.await?
			},
		};
		Ok(identifier)
	}
}

impl Instance {
	#[allow(clippy::unused_async)]
	pub async fn resolve_module_with_path_specifier(
		specifier: &Path,
		referrer: &Identifier,
	) -> Result<Identifier> {
		match referrer {
			Identifier::Normal(referrer) => {
				let mut path = referrer.path.clone();
				path.parent();
				path.join(specifier.clone());

				// If the path ends in `.tg`, then it specifies a normal module. Otherwise, it specifies an artifact module.
				if path.extension() == Some("tg") {
					Ok(Identifier::Normal(identifier::Normal {
						source: referrer.source.clone(),
						path,
					}))
				} else {
					Ok(Identifier::Artifact(identifier::Artifact {
						source: referrer.source.clone(),
						path,
					}))
				}
			},

			Identifier::Artifact(_) => {
				bail!("Artifact modules cannot have imports.");
			},

			Identifier::Lib(referrer) => {
				let mut path = referrer.path.clone();
				path.parent();
				path.join(specifier.clone());
				Ok(Identifier::Lib(Lib { path }))
			},
		}
	}

	async fn resolve_module_with_dependency_specifier(
		&self,
		specifier: &dependency::Specifier,
		referrer: &Identifier,
	) -> Result<Identifier> {
		// Convert the module dependency specifier to a package dependency specifier.
		let specifier = specifier.to_package_dependency_specifier(referrer)?;

		match referrer {
			Identifier::Normal(identifier::Normal {
				source: Source::Path(package_path),
				..
			}) => {
				self.resolve_module_with_dependency_specifier_from_path_referrer(
					&specifier,
					package_path,
				)
				.await
			},

			Identifier::Normal(identifier::Normal {
				source: Source::Instance(package_instance_hash),
				..
			}) => {
				self.resolve_module_with_dependency_specifier_from_instance_referrer(
					&specifier,
					*package_instance_hash,
				)
				.await
			},

			_ => bail!(r#"Cannot resolve a package specifier from referrer "{referrer}"."#),
		}
	}

	#[allow(clippy::unused_async)]
	async fn resolve_module_with_dependency_specifier_from_path_referrer(
		&self,
		specifier: &package::dependency::Specifier,
		referrer_package_path: &os::Path,
	) -> Result<Identifier> {
		match specifier {
			package::dependency::Specifier::Path(specifier_path) => {
				let specifier_path: os::PathBuf = specifier_path.clone().into();
				let package_path = referrer_package_path.join(specifier_path);
				let identifier = Identifier::for_root_module_in_package_at_path(&package_path);
				Ok(identifier)
			},

			package::dependency::Specifier::Registry(_) => todo!(),
		}
	}

	#[allow(clippy::unused_async)]
	async fn resolve_module_with_dependency_specifier_from_instance_referrer(
		&self,
		specifier: &package::dependency::Specifier,
		referrer_package_instance_hash: package::instance::Hash,
	) -> Result<Identifier> {
		// Get the referrer package.
		let referrer = self.get_package_instance_local(referrer_package_instance_hash)?;

		// Get the specifier's package instance hash from the referrer's dependencies.
		let specifier_package_instance_hash = referrer
			.dependencies
			.get(specifier)
			.context("Expected the referrer's dependencies to contain the specifier.")?;

		// Create the module identifier.
		let module_identifier =
			Identifier::for_root_module_in_package_instance(*specifier_package_instance_hash);

		Ok(module_identifier)
	}
}
