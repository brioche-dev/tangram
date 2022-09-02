use super::{Client, Transport};
use crate::{expression::Expression, hash::Hash, value::Value};
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, expression: Expression) -> Result<Value> {
		match &self.transport {
			Transport::InProcess(server) => {
				let value = server.evaluate(expression).await?;
				Ok(value)
			},
			Transport::Unix(transport) => {
				let expression_json = serde_json::to_vec(&expression)?;
				let expression_hash = Hash::new(&expression_json);
				let value = transport
					.post_json(
						&format!("/epxressions/{expression_hash}/evaluate"),
						&expression,
					)
					.await?;
				Ok(value)
			},
			Transport::Tcp(transport) => {
				let expression_json = serde_json::to_vec(&expression)?;
				let expression_hash = Hash::new(&expression_json);
				let value = transport
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
