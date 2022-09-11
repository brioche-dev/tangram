use super::Client;
use crate::{
	expression::{self, Expression},
	hash,
};
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, expression: &Expression) -> Result<Expression> {
		let expression_json = serde_json::to_vec(expression)?;
		let expression_hash = expression::Hash(hash::Hash::new(&expression_json));
		match self.transport.as_in_process_or_http() {
			super::transport::InProcessOrHttp::InProcess(server) => {
				let output = server.evaluate(expression, expression_hash).await?;
				Ok(output)
			},
			super::transport::InProcessOrHttp::Http(http) => {
				let output = http
					.post_json(
						&format!("/expressions/{expression_hash}/evaluate"),
						expression,
					)
					.await?;
				Ok(output)
			},
		}
	}
}
