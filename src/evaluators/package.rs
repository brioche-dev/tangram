use std::sync::Arc;

use crate::{builder, evaluators::Evaluator, expression::Expression, hash::Hash};
use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;

pub struct Package;

impl Package {
	#[must_use]
	pub fn new() -> Package {
		Package {}
	}
}

impl Default for Package {
	fn default() -> Self {
		Package::new()
	}
}

#[async_trait]
impl Evaluator for Package {
	async fn evaluate(
		&self,
		builder: &builder::Shared,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let package = if let Expression::Package(package) = expression {
			package
		} else {
			return Ok(None);
		};

		// Evaluate the source.
		let source = builder.evaluate(package.source, hash).await?;

		// Evaluate the dependencies.
		let dependencies = package
			.dependencies
			.iter()
			.map(|(name, dependency)| async move {
				let dependency = builder.evaluate(*dependency, hash).await?;
				Ok::<_, anyhow::Error>((Arc::clone(name), dependency))
			});
		let dependencies = try_join_all(dependencies).await?.into_iter().collect();

		let output = Expression::Package(crate::expression::Package {
			source,
			dependencies,
		});
		let output_hash = builder.add_expression(&output).await?;

		Ok(Some(output_hash))
	}
}
