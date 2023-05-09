use super::{Instance, Package};
use crate::{
	error::{Result, WrapErr},
	module,
};
use async_recursion::async_recursion;
use std::{
	collections::{BTreeMap, HashSet},
	sync::Arc,
};

impl Package {
	/// Instantiate the package.
	#[async_recursion]
	pub async fn instantiate(
		&self,
		tg: &'async_recursion Arc<crate::instance::Instance>,
	) -> Result<Instance> {
		// Get the package's dependency specifiers.
		let mut package_dependency_specifiers = HashSet::new();
		for (module_path, analyze_output) in self
			.analyze(tg)
			.await
			.wrap_err("Failed to analyze the package.")?
		{
			// Add the package dependency specifiers.
			for import in &analyze_output.imports {
				if let module::Specifier::Dependency(specifier) = import {
					// Convert the module dependency specifier to a package dependency specifier.
					let package_dependency_specifier = match specifier {
						module::dependency::Specifier::Path(path) => {
							let path = module_path
								.clone()
								.into_relpath()
								.parent()
								.join(path.clone());
							super::dependency::Specifier::Path(path)
						},
						module::dependency::Specifier::Registry(registry) => {
							super::dependency::Specifier::Registry(registry.clone())
						},
					};
					package_dependency_specifiers.insert(package_dependency_specifier);
				}
			}
		}

		// Instantiate the dependencies.
		let mut dependencies = BTreeMap::default();
		for package_dependency_specifier in package_dependency_specifiers {
			// Resolve the package from the package dependency specifier.
			let package = match &package_dependency_specifier {
				super::dependency::Specifier::Path(path) => {
					let path = self
						.path
						.as_ref()
						.wrap_err("The package must have a path.")?
						.join(path.to_string());
					Self::check_in(tg, &path).await?
				},
				super::dependency::Specifier::Registry(_) => {
					todo!()
				},
			};

			// Instantiate the dependency.
			let package_instance = package.instantiate(tg).await?;

			// Add the dependency to the dependencies.
			dependencies.insert(package_dependency_specifier, package_instance);
		}

		// Create the package instance.
		let package_instance = Instance::new(tg, self.clone(), dependencies).await?;

		Ok(package_instance)
	}
}
