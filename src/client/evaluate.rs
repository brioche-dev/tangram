use super::{Client, Transport};
use crate::{expression::Expression, value::Value};
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, expression: Expression) -> Result<Value> {
		match &self.transport {
			Transport::InProcess { server, .. } => server.evaluate(expression).await,
			_ => self.post_json("/evaluate", &expression).await,
		}
	}
}
