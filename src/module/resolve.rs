use super::{dependency, Document, Library, Module, Specifier};
use crate::{
	error::{return_error, Result, WrapErr},
	instance::Instance,
	package::{self, ROOT_MODULE_FILE_NAME},
	path,
};

impl Module {
	/// Resolve a specifier relative to the module.
	pub async fn resolve(&self, tg: &Instance, specifier: &Specifier) -> Result<Self> {
		match (self, specifier) {
			(Self::Library(module), Specifier::Path(specifier)) => {
				let module_path = module
					.module_path
					.clone()
					.join(path::Component::Parent)
					.join(specifier.clone());
				Ok(Self::Library(Library { module_path }))
			},

			(Self::Library(_), Specifier::Dependency(_)) => {
				return_error!(r#"Cannot resolve a dependency specifier from a library module."#);
			},

			(Self::Document(document), Specifier::Path(specifier)) => {
				// Resolve the module path.
				let package_path = document.package_path.clone();
				let module_path = document
					.module_path
					.clone()
					.join(path::Component::Parent)
					.join(specifier.clone());

				// Ensure that the module path is within the package.
				if module_path.has_parent_components() {
					return_error!(
						r#"Cannot resolve a path specifier to a module outside of the package."#
					);
				}

				// Ensure that the module exists.
				let path = package_path.join(module_path.to_string());
				let exists = tokio::fs::try_exists(&path).await?;
				if !exists {
					let path = path.display();
					return_error!(r#"Could not find a module at path "{path}"."#);
				}

				// Create the module.
				let module = Self::Document(Document::new(tg, package_path, module_path).await?);

				Ok(module)
			},

			(
				Self::Document(document),
				Specifier::Dependency(dependency::Specifier::Path(specifier)),
			) => {
				// Convert the module dependency specifier to a package dependency specifier.
				let specifier = document
					.module_path
					.clone()
					.join(path::Component::Parent)
					.join(specifier.clone());

				// Resolve the package path.
				let package_path = document.package_path.join(specifier.to_string());
				let package_path = tokio::fs::canonicalize(package_path).await?;

				// The module path is the root module.
				let module_path = ROOT_MODULE_FILE_NAME.into();

				Ok(Self::Document(
					Document::new(tg, package_path, module_path).await?,
				))
			},

			(Self::Document(_), Specifier::Dependency(dependency::Specifier::Registry(_))) => {
				todo!()
			},

			(Self::Normal(module), Specifier::Path(specifier)) => {
				let module_path = module
					.module_path
					.clone()
					.join(path::Component::Parent)
					.join(specifier.clone());
				Ok(Self::Normal(super::Normal {
					package_instance_hash: module.package_instance_hash,
					module_path,
				}))
			},

			(Self::Normal(module), Specifier::Dependency(specifier)) => {
				// Convert the module dependency specifier to a package dependency specifier.
				let package_dependency_specifier = match specifier {
					dependency::Specifier::Path(specifier) => {
						let path = module
							.module_path
							.clone()
							.join(path::Component::Parent)
							.join(specifier.clone());
						package::dependency::Specifier::Path(path)
					},
					dependency::Specifier::Registry(registry) => {
						package::dependency::Specifier::Registry(registry.clone())
					},
				};

				// Get the package instance.
				let package_instance =
					package::Instance::get(tg, module.package_instance_hash).await?;

				// Get the specified package instance from the dependencies.
				let dependencies = package_instance.dependencies(tg).await?;
				let package_instance = dependencies
					.get(&package_dependency_specifier)
					.wrap_err("Expected the dependencies to contain the dependency specifier.")?;

				// Get the root module.
				let module = package_instance.root_module();

				Ok(module)
			},
		}
	}
}
