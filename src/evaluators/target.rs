use crate::{
	builder,
	evaluators::Evaluator,
	expression::{self, Expression},
	hash::Hash,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use camino::Utf8PathBuf;

pub struct Target;

impl Target {
	#[must_use]
	pub fn new() -> Target {
		Target {}
	}
}

impl Default for Target {
	fn default() -> Self {
		Target::new()
	}
}

#[async_trait]
impl Evaluator for Target {
	async fn evaluate(
		&self,
		builder: &builder::Shared,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let target = if let Expression::Target(target) = expression {
			target
		} else {
			return Ok(None);
		};

		// Get the package's dependencies.
		let package = builder
			.get_expression(target.package)
			.await?
			.into_package()
			.ok_or_else(|| anyhow!("Expected a package expression."))?;

		// Add the js process expression.
		let expression_hash = builder
			.add_expression(&expression::Expression::Js(expression::Js {
				dependencies: package.dependencies,
				artifact: target.package,
				path: Some(Utf8PathBuf::from("tangram.js")),
				name: target.name.clone(),
				args: target.args,
			}))
			.await?;

		// Evaluate the expression.
		let output = builder.evaluate(expression_hash, hash).await?;

		Ok(Some(output))
	}
}
