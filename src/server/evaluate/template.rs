use crate::{
	expression::{self, Expression},
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
		root_expression_hash: expression::Hash,
	) -> Result<Expression> {
		let components = template
			.components
			.iter()
			.map(|component| self.evaluate(component, root_expression_hash));
		let components = try_join_all(components).await?;
		let output = Expression::Template(crate::expression::Template { components });
		Ok(output)
	}
}
