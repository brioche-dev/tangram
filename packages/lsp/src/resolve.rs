use super::ROOT_MODULE_FILE_NAME;
use crate::{
	document,
	module::{Library, Normal},
	Document, Import, Module,
};
use tangram_client as tg;
use tangram_error::{error, return_error, Result, WrapErr};

impl Module {
	/// Resolve a module.
	#[allow(clippy::too_many_lines)]
	pub async fn resolve(
		&self,
		client: &dyn tg::Client,
		document_store: Option<&document::Store>,
		import: &Import,
	) -> Result<Self> {
		match (self, import) {
			(Self::Library(module), Import::Module(path)) => {
				let path = module.path.clone().parent().join(path.clone()).normalize();
				Ok(Self::Library(Library { path }))
			},

			(Self::Library(_), Import::Dependency(_)) => Err(error!(
				r#"Cannot resolve a dependency import from a library module."#
			)),

			(Self::Document(document), Import::Module(path)) => {
				// Resolve the module path.
				let package_path = document.package_path.clone();
				let module_subpath = document
					.path
					.clone()
					.parent()
					.join(path.clone())
					.normalize();

				// Ensure that the module exists.
				let module_path = package_path.join(module_subpath.to_string());
				let exists = tokio::fs::try_exists(&module_path)
					.await
					.wrap_err("Failed to determine if the path exists.")?;
				if !exists {
					let path = module_path.display();
					return_error!(r#"Could not find a module at path "{path}"."#);
				}

				// Create the document.
				let document =
					Document::new(document_store.unwrap(), package_path, module_subpath).await?;

				// Create the module.
				let module = Self::Document(document);

				Ok(module)
			},

			(Self::Document(document), Import::Dependency(dependency))
				if dependency.path.is_some() =>
			{
				// Resolve the package path.
				let dependency_path = document
					.path
					.clone()
					.parent()
					.join(dependency.path.as_ref().unwrap().clone())
					.normalize();

				let package_path = document.package_path.join(dependency_path.to_string());
				let package_path = tokio::fs::canonicalize(package_path)
					.await
					.wrap_err("Failed to canonicalize the path.")?;

				// The module path is the root module.
				let module_path = ROOT_MODULE_FILE_NAME.parse().unwrap();

				// Create the document.
				let document =
					Document::new(document_store.unwrap(), package_path, module_path).await?;

				// Create the module.
				let module = Self::Document(document);

				Ok(module)
			},

			(Self::Document(document), Import::Dependency(dependency)) => {
				// Convert the module dependency to a package dependency.
				let module_subpath = document.path.clone();
				let dependency = match &dependency.path {
					Some(dependency_path) => tg::Dependency::with_path(
						module_subpath
							.parent()
							.join(dependency_path.clone())
							.normalize(),
					),
					None => dependency.clone(),
				};

				// Get the lock for the document's package.
				let path = document.package_path.clone().try_into()?;
				let dependency_ = tg::Dependency::with_path(path);
				let (_, lock) = client.create_package_and_lock(&dependency_).await?;
				let lock = tg::Lock::with_id(lock);

				// Get the lock entry for the dependency.
				let Some(entry) = lock.dependencies(client).await?.get(&dependency) else {
					return_error!("Could not find the dependency.");
				};

				// Create the module.
				let lock = lock.id(client).await?.clone();
				let package = entry.package.id(client).await?.clone();
				let path = ROOT_MODULE_FILE_NAME.parse().unwrap();
				let module = Self::Normal(Normal {
					lock,
					package,
					path,
				});

				Ok(module)
			},

			(Self::Normal(module), Import::Module(path)) => {
				let path = module.path.clone().parent().join(path.clone()).normalize();
				Ok(Self::Normal(Normal {
					package: module.package.clone(),
					path,
					lock: module.lock.clone(),
				}))
			},

			(Self::Normal(module), Import::Dependency(dependency)) => {
				// Convert the module dependency to a package dependency.
				let module_subpath = module.path.clone();
				let dependency = match &dependency.path {
					Some(dependency_path) => tg::Dependency::with_path(
						module_subpath
							.parent()
							.join(dependency_path.clone())
							.normalize(),
					),
					None => dependency.clone(),
				};

				// Get this module's lock.
				let parent_lock = tg::Lock::with_id(module.lock.clone());

				// Get the specified package and lock from the dependencies.
				let dependencies = parent_lock.dependencies(client).await?;
				let tg::lock::Entry { package, lock } =
					dependencies.get(&dependency).cloned().ok_or_else(|| {
						error!("Could not find {dependency} in lock ({parent_lock}).")
					})?;

				// Create the module.
				let module = Module::Normal(Normal {
					package: package.id(client).await?.clone(),
					path: ROOT_MODULE_FILE_NAME.parse().unwrap(),
					lock: lock.id(client).await?.clone(),
				});

				Ok(module)
			},
		}
	}
}
