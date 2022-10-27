use crate::{
	builder::State,
	expression::{self, Target},
	hash::Hash,
	system::System,
};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

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

		// If the target name is "shell", and the package does not define a "shell" target, construct one.
		if target.name == "shell" && !manifest.targets.iter().any(|name| name == "shell") {
			let std_package_hash = package
				.dependencies
				.get("std")
				.copied()
				.context("The package must have a dependency on std.")?;

			// Create the dependency args.
			let mut dependency_arg = BTreeMap::new();
			let system = System::host()?;
			let system = self
				.add_expression(&expression::Expression::String(system.to_string().into()))
				.await?;
			dependency_arg.insert("system".into(), system);
			let dependency_arg = self
				.add_expression(&expression::Expression::Map(dependency_arg))
				.await?;
			let dependency_args = self
				.add_expression(&expression::Expression::Array(vec![dependency_arg]))
				.await?;

			// Create the target expressions for the dependencies.
			let mut dependencies = Vec::new();
			for (dependency_name, dependency_hash) in package.dependencies {
				if dependency_name.as_ref() != "std" {
					dependencies.push(
						self.add_expression(&expression::Expression::Target(expression::Target {
							package: dependency_hash,
							name: "default".to_owned(),
							args: dependency_args,
						}))
						.await?,
					);
				}
			}
			let dependencies = self
				.add_expression(&expression::Expression::Array(dependencies))
				.await?;

			// Create the args.
			let arg = self
				.add_expression(&expression::Expression::Map(
					[("dependencies".into(), dependencies)].into(),
				))
				.await?;
			let args = self
				.add_expression(&expression::Expression::Array(vec![arg]))
				.await?;

			// Add the target expression.
			let expression_hash = self
				.add_expression(&expression::Expression::Target(expression::Target {
					package: std_package_hash,
					name: "shell".to_owned(),
					args,
				}))
				.await?;

			// Evaluate the expression.
			let output = self.evaluate(expression_hash, hash).await?;

			return Ok(output);
		}

		// Get the package's js entrypoint path.
		let js_entrypoint_path = self
			.get_package_js_entrypoint(package_hash)
			.context("Failed to retrieve the package JS entrypoint.")?
			.context("The package must have a JS entrypoint.")?;

		// Add the js process expression.
		let expression_hash = self
			.add_expression(&expression::Expression::Js(expression::Js {
				package: target.package,
				path: js_entrypoint_path,
				name: target.name.clone(),
				args: target.args,
			}))
			.await?;

		// Evaluate the expression.
		let output = self.evaluate(expression_hash, hash).await?;

		Ok(output)
	}
}
