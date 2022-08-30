use super::{Client, Transport};
use crate::{expression::Expression, value::Value};
use anyhow::Result;

impl Client {
	pub async fn evaluate(&self, expression: Expression) -> Result<Value> {
		match &self.transport {
			Transport::InProcess(server) => {
				let value = server.evaluate(expression).await?;
				Ok(value)
			},
			Transport::Unix(transport) => {
				let value = transport.post_json("/evaluate", &expression).await?;
				Ok(value)
			},
			Transport::Tcp(transport) => {
				let value = transport.post_json("/evaluate", &expression).await?;
				Ok(value)
			},
		}
	}
}
