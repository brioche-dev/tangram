use crate::{
	expression::{Expression, Package, Target},
	hash::Hash,
	system::System,
	State,
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

mod js;

impl State {
	pub(super) async fn evaluate_target(&self, hash: Hash, target: &Target) -> Result<Hash> {
		// Evaluate the package.
		let package_hash = self
			.evaluate(target.package, hash)
			.await
			.context("Failed to evaluate the package.")?;

		// Get the package.
		let package = self
			.get_expression_local(package_hash)?
			.into_package()
			.context("Expected a package expression.")?;

		// Read the package manifest and get the list of targets.
		let manifest = self
			.get_package_manifest(package_hash)
			.await
			.context("Failed to get the package manifest.")?;

		// If the target name is "shell", and the package does not define a "shell" target, create and evaluate a target expression that invokes the "shell" target from std.
		if target.name == "shell" && !manifest.targets.iter().any(|name| name == "shell") {
			let expression = self.create_default_shell_expression(&package).await?;

			// Add the expression.
			let expression_hash = self.add_expression(&expression).await?;

			// Evaluate the expression.
			let output_hash = self.evaluate(expression_hash, hash).await?;

			return Ok(output_hash);
		}

		let output_hash = self.evaluate_target_js(hash, target).await?;

		Ok(output_hash)
	}

	async fn create_default_shell_expression(&self, package: &Package) -> Result<Expression> {
		let std_package_hash = package
			.dependencies
			.get("std")
			.copied()
			.context("The package must have a dependency on std.")?;

		// Create the dependency args.
		let mut dependency_arg = BTreeMap::new();
		let system = System::host()?;
		let system = self
			.add_expression(&Expression::String(system.to_string().into()))
			.await?;
		dependency_arg.insert("system".into(), system);
		let dependency_arg = self
			.add_expression(&Expression::Map(dependency_arg))
			.await?;
		let dependency_args = self
			.add_expression(&Expression::Array(vec![dependency_arg]))
			.await?;

		// Create the target expressions for the dependencies.
		let mut dependency_targets = Vec::new();
		for (dependency_name, dependency_hash) in &package.dependencies {
			if dependency_name.as_ref() == "std" {
				continue;
			}
			let dependency_target_hash = self
				.add_expression(&Expression::Target(Target {
					package: *dependency_hash,
					name: "default".to_owned(),
					args: dependency_args,
				}))
				.await?;
			dependency_targets.push(dependency_target_hash);
		}
		let dependencies = self
			.add_expression(&Expression::Array(dependency_targets))
			.await?;

		// Create the args.
		let arg = self
			.add_expression(&Expression::Map(
				[("dependencies".into(), dependencies)].into(),
			))
			.await?;
		let args = self.add_expression(&Expression::Array(vec![arg])).await?;

		// Create the expression.
		let expression = Expression::Target(Target {
			package: std_package_hash,
			name: "shell".to_owned(),
			args,
		});

		Ok(expression)
	}
}
