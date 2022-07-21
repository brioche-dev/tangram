use crate::{
	expression,
	server::Server,
	value::{self, Value},
};
use anyhow::Result;
use std::sync::Arc;

impl Server {
	/// Evaluate a template expression.
	pub async fn evaluate_template(
		self: &Arc<Self>,
		template: expression::Template,
	) -> Result<Value> {
		let components = template
			.components
			.into_iter()
			.map(|component| self.evaluate(component));
		let components = futures::future::try_join_all(components).await?;
		let value = Value::Template(value::Template { components });
		Ok(value)
	}
}
