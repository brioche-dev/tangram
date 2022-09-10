use crate::{
	expression::{self, Expression},
	server::Server,
};
use anyhow::Result;
use std::sync::Arc;

impl Server {
	/// Evaluate a template expression.
	pub async fn evaluate_template(
		self: &Arc<Self>,
		template: &expression::Template,
	) -> Result<Expression> {
		let components = template
			.components
			.iter()
			.map(|component| self.evaluate(component));
		let components = futures::future::try_join_all(components).await?;
		let output = Expression::Template(crate::expression::Template { components });
		Ok(output)
	}
}
