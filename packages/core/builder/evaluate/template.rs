use crate::{
	builder::Shared,
	expression::{self, Expression, Template},
	hash::Hash,
};
use anyhow::Result;
use futures::future::try_join_all;

impl Shared {
	/// Evaluate a template expression.
	pub(super) async fn evaluate_template(&self, hash: Hash, template: &Template) -> Result<Hash> {
		let components = template
			.components
			.iter()
			.copied()
			.map(|component| self.evaluate(component, hash));
		let components = try_join_all(components).await?;
		let output = Expression::Template(expression::Template { components });
		let output_hash = self.add_expression(&output).await?;
		Ok(output_hash)
	}
}
