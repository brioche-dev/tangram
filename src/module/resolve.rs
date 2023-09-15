use super::{Import, Library, Module};
use crate::{
	document::Document,
	package::{Dependency, ROOT_MODULE_FILE_NAME},
	return_error, Client, Package, Result, Server, WrapErr,
};

impl Module {
	/// Resolve a module.
	#[allow(clippy::too_many_lines)]
	pub async fn resolve(&self, server: &Server, import: &Import) -> Result<Self> {
		match (self, import) {
			(Self::Library(module), Import::Path(path)) => {
				let path = module
					.path
					.clone()
					.into_relpath()
					.parent()
					.join(path.clone())
					.try_into_subpath()
					.wrap_err("Failed to resolve the module path.")?;
				Ok(Self::Library(Library { path }))
			},

			(Self::Library(_), Import::Dependency(_)) => {
				return_error!(r#"Cannot resolve a dependency import from a library module."#);
			},

			(Self::Document(document), Import::Path(path)) => {
				// Resolve the module path.
				let package_path = document.package_path.clone();
				let module_subpath = document
					.module_path
					.clone()
					.into_relpath()
					.parent()
					.join(path.clone())
					.try_into_subpath()
					.wrap_err("Failed to resolve the module path.")?;

				// Ensure that the module exists.
				let module_path = package_path.join(module_subpath.to_string());
				let exists = tokio::fs::try_exists(&module_path).await?;
				if !exists {
					let path = module_path.display();
					return_error!(r#"Could not find a module at path "{path}"."#);
				}

				// Create the module.
				let module =
					Self::Document(Document::new(server, package_path, module_subpath).await?);

				Ok(module)
			},

			(Self::Document(document), Import::Dependency(Dependency::Path(dependency_path))) => {
				// Resolve the package path.
				let dependency_path = document
					.module_path
					.clone()
					.into_relpath()
					.parent()
					.join(dependency_path.clone());
				let package_path = document.package_path.join(dependency_path.to_string());
				let package_path = tokio::fs::canonicalize(package_path).await?;

				// The module path is the root module.
				let module_path = ROOT_MODULE_FILE_NAME.parse().unwrap();

				Ok(Self::Document(
					Document::new(server, package_path, module_path).await?,
				))
			},

			(Self::Document(_), Import::Dependency(Dependency::Registry(_))) => {
				unimplemented!()
			},

			(Self::Normal(module), Import::Path(path)) => {
				let path = module
					.path
					.clone()
					.into_relpath()
					.parent()
					.join(path.clone())
					.try_into_subpath()
					.wrap_err("Failed to resolve the module path.")?;
				Ok(Self::Normal(super::Normal {
					package: module.package,
					path,
				}))
			},

			(Self::Normal(module), Import::Dependency(dependency)) => {
				// Convert the module dependency to a package dependency.
				let module_subpath = module.path.clone();
				let dependency = match dependency {
					Dependency::Path(dependency_path) => Dependency::Path(
						module_subpath
							.into_relpath()
							.parent()
							.join(dependency_path.clone()),
					),
					Dependency::Registry(_) => dependency.clone(),
				};

				// Get the package.
				let package = Package::with_id(module.package);

				// Get the specified package from the dependencies.
				let client = &Client::with_server(server.clone());
				let dependencies = package
					.dependencies(client)
					.await?
					.as_ref()
					.wrap_err("Expected the package to be locked.")?;
				let package = dependencies
					.get(&dependency)
					.cloned()
					.wrap_err("Expected the dependencies to contain the dependency.")?;

				// Get the root module.
				let module = package.root_module(client).await?;

				Ok(module)
			},
		}
	}
}
