use crate::{
	expression::{self, Expression},
	hash::Hash,
	server::Server,
};
use anyhow::Result;
use futures::future::try_join_all;
use std::sync::Arc;

impl Server {
	/// Evaluate a template expression.
	pub async fn evaluate_template(
		self: &Arc<Self>,
		template: &expression::Template,
		parent_hash: Hash,
	) -> Result<Hash> {
		let components = template
			.components
			.iter()
			.copied()
			.map(|component| self.evaluate(component, parent_hash));
		let components = try_join_all(components).await?;
		let output = Expression::Template(crate::expression::Template { components });
		let output_hash = self.add_expression(&output).await?;
		Ok(output_hash)
	}
}
