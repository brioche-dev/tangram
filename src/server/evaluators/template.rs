use crate::{
	expression::Expression,
	hash::Hash,
	server::{Evaluator, Server},
};
use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;
use std::sync::Arc;

pub struct Template;

impl Template {
	#[must_use]
	pub fn new() -> Template {
		Template {}
	}
}

impl Default for Template {
	fn default() -> Self {
		Template::new()
	}
}

#[async_trait]
impl Evaluator for Template {
	/// Evaluate a template expression.
	async fn evaluate(
		&self,
		server: &Arc<Server>,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let template = if let Expression::Template(template) = expression {
			template
		} else {
			return Ok(None);
		};
		let components = template
			.components
			.iter()
			.copied()
			.map(|component| server.evaluate(component, hash));
		let components = try_join_all(components).await?;
		let output = Expression::Template(crate::expression::Template { components });
		let output_hash = server.add_expression(&output).await?;
		Ok(Some(output_hash))
	}
}
