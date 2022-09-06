use super::Client;
use crate::{expression::Expression, hash::Hash, value::Value};
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, expression: Expression) -> Result<Value> {
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let value = server.evaluate(expression).await?;
				Ok(value)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let expression_json = serde_json::to_vec(&expression)?;
				let expression_hash = Hash::new(&expression_json);
				let value = http
					.post_json(
						&format!("/expressions/{expression_hash}/evaluate"),
						&expression,
					)
					.await?;
				Ok(value)
			},
		}
	}
}
