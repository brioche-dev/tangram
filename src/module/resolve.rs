use super::{dependency, identifier::Source, Identifier, Specifier};
use crate::{
	error::{return_error, Result, WrapErr},
	package,
	path::{self, Path},
	util::fs,
	Instance,
};

impl Instance {
	/// Resolve a specifier relative to a referrer.
	pub async fn resolve_module(
		&self,
		specifier: &Specifier,
		referrer: &Identifier,
	) -> Result<Identifier> {
		let identifier = match specifier {
			Specifier::Path(path_specifier) => {
				Self::resolve_module_with_path_specifier(path_specifier, referrer).await?
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
		let path = referrer
			.path
			.clone()
			.join([path::Component::ParentDir])
			.join(specifier.clone());
		Ok(Identifier {
			source: referrer.source.clone(),
			path,
		})
	}

	async fn resolve_module_with_dependency_specifier(
		&self,
		specifier: &dependency::Specifier,
		referrer: &Identifier,
	) -> Result<Identifier> {
		// Convert the module dependency specifier to a package dependency specifier.
		let specifier = specifier.to_package_dependency_specifier(referrer)?;

		match referrer {
			Identifier {
				source: Source::Package(package_path),
				..
			} => {
				self.resolve_module_with_dependency_specifier_from_package_referrer(
					&specifier,
					package_path,
				)
				.await
			},

			Identifier {
				source: Source::PackageInstance(package_instance_hash),
				..
			} => {
				self.resolve_module_with_dependency_specifier_from_package_instance_referrer(
					&specifier,
					*package_instance_hash,
				)
				.await
			},

			_ => return_error!(r#"Cannot resolve a package specifier from referrer "{referrer}"."#),
		}
	}

	#[allow(clippy::unused_async)]
	async fn resolve_module_with_dependency_specifier_from_package_referrer(
		&self,
		specifier: &package::dependency::Specifier,
		referrer_package_identifier: &package::Identifier,
	) -> Result<Identifier> {
		match (specifier, referrer_package_identifier) {
			(
				package::dependency::Specifier::Path(specifier_path),
				package::Identifier::Path(referrer_package_path),
			) => {
				let specifier_path: fs::PathBuf = specifier_path.clone().into();
				let package_path = referrer_package_path.join(specifier_path);
				let package_identifier = package::Identifier::Path(package_path);
				let module_identifier = Identifier::for_root_module_in_package(package_identifier);
				Ok(module_identifier)
			},

			_ => todo!(),
		}
	}

	#[allow(clippy::unused_async)]
	async fn resolve_module_with_dependency_specifier_from_package_instance_referrer(
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
			.wrap_err("Expected the referrer's dependencies to contain the specifier.")?;

		// Create the module identifier.
		let module_identifier =
			Identifier::for_root_module_in_package_instance(*specifier_package_instance_hash);

		Ok(module_identifier)
	}
}
