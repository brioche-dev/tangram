use crate::{
	expression::{self, Expression, Package},
	hash::Hash,
	State,
};
use anyhow::Result;
use futures::future::try_join_all;
use std::sync::Arc;

impl State {
	pub(super) async fn evaluate_package(&self, hash: Hash, package: &Package) -> Result<Hash> {
		// Evaluate the source.
		let source = self.evaluate(package.source, hash).await?;

		// Evaluate the dependencies.
		let dependencies = package
			.dependencies
			.iter()
			.map(|(name, dependency)| async move {
				let dependency = self.evaluate(*dependency, hash).await?;
				Ok::<_, anyhow::Error>((Arc::clone(name), dependency))
			});
		let dependencies = try_join_all(dependencies).await?.into_iter().collect();

		let output = Expression::Package(expression::Package {
			source,
			dependencies,
		});
		let output_hash = self.add_expression(&output).await?;

		Ok(output_hash)
	}
}
