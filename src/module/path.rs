use super::Module;
use crate::{error::Result, instance::Instance, package};
use std::path::PathBuf;

impl Module {
	pub async fn path(&self, tg: &Instance) -> Result<Option<PathBuf>> {
		match self {
			Module::Library(_) => Ok(None),

			Module::Document(document) => Ok(Some(document.path())),

			Module::Normal(module) => {
				// Get the package.
				let package_instance =
					package::Instance::get(tg, module.package_instance_hash).await?;
				let package = package_instance.package();

				// Get the package path.
				let packages = tg.modules.packages.read().unwrap();
				let Some(specifier) = packages.get(package) else {
					return Ok(None);
				};
				let package::Specifier::Path(package_path) = specifier else {
					return Ok(None);
				};

				// Get the path.
				let path = package_path.join(module.module_path.to_string());

				Ok(Some(path))
			},
		}
	}
}
