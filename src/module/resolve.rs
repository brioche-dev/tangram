use super::{
	identifier::{self, Lib, Source},
	Identifier, Specifier,
};
use crate::{os, package, path::Path, Cli};
use anyhow::{bail, Context, Result};

impl Cli {
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
			Specifier::Package(package_specifier) => {
				self.resolve_module_with_package_specifier(package_specifier, referrer)
					.await?
			},
		};
		Ok(identifier)
	}
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn resolve_module_with_path_specifier(
		specifier: &Path,
		referrer: &Identifier,
	) -> Result<Identifier> {
		match referrer {
			Identifier::Normal(referrer) => {
				let path = referrer.path.parent().join(specifier).normalize();

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
				let path = referrer.path.parent().join(specifier).normalize();
				Ok(Identifier::Lib(Lib { path }))
			},
		}
	}

	async fn resolve_module_with_package_specifier(
		&self,
		specifier: &package::Specifier,
		referrer: &Identifier,
	) -> Result<Identifier> {
		match referrer {
			Identifier::Normal(identifier::Normal {
				source: Source::Path(package_path),
				path,
			}) => {
				self.resolve_module_with_package_path_specifier(specifier, package_path, path)
					.await
			},

			Identifier::Normal(identifier::Normal {
				source: Source::Instance(package_instance_hash),
				path,
			}) => {
				self.resolve_module_with_package_instance_specifier(
					specifier,
					*package_instance_hash,
					path,
				)
				.await
			},

			_ => bail!(r#"Cannot resolve package from referrer "{referrer}"."#),
		}
	}

	#[allow(clippy::unused_async)]
	async fn resolve_module_with_package_path_specifier(
		&self,
		specifier: &package::Specifier,
		referrer_package_path: &os::Path,
		referrer_path: &Path,
	) -> Result<Identifier> {
		match specifier {
			package::Specifier::Path(path) => {
				let package_path = referrer_package_path
					.join(referrer_path.to_string())
					.join("..")
					.join(path);
				let identifier = Identifier::for_root_module_in_package_at_path(&package_path);
				Ok(identifier)
			},

			package::Specifier::Registry(_) => todo!(),
		}
	}

	#[allow(clippy::unused_async)]
	async fn resolve_module_with_package_instance_specifier(
		&self,
		specifier: &package::Specifier,
		referrer_package_instance_hash: package::instance::Hash,
		referrer_path: &Path,
	) -> Result<Identifier> {
		// Get the specifier name.
		let specifier_name = match specifier {
			package::Specifier::Path(specifier_path) => {
				let specifier_path = specifier_path.display().to_string().into();
				referrer_path.join(&specifier_path).normalize().to_string()
			},

			package::Specifier::Registry(specifier) => specifier.to_string(),
		};

		// Get the referrer.
		let referrer = self.get_package_instance_local(referrer_package_instance_hash)?;

		// Get the specifier's package instance hash from the referrer's dependencies.
		let specifier_package_instance_hash = referrer
			.dependencies
			.get(&specifier_name)
			.context("Expected the referrer's dependencies to contain the specifier.")?;

		// Create the module identifier.
		let module_identifier =
			Identifier::for_root_module_in_package_instance(*specifier_package_instance_hash);

		Ok(module_identifier)
	}
}
