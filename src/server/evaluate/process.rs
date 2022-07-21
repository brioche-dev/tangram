use crate::server::Server;
use crate::{expression, value::Value};
use anyhow::Result;
use std::sync::Arc;

impl Server {
	pub async fn evaluate_process(
		self: &Arc<Self>,
		_expression: expression::Process,
	) -> Result<Value> {
		todo!()
	}
}
