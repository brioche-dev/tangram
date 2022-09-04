use crate::{
	expression,
	server::{runtime, Server},
	value::Value,
};
use anyhow::{Context, Result};
use std::sync::Arc;

impl Server {
	pub async fn evaluate_js_process(
		self: &Arc<Self>,
		process: expression::JsProcess,
	) -> Result<Value> {
		// Create a JS runtime.
		let runtime = runtime::js::Runtime::new(self);
		// Run the process.
		let expression = runtime
			.run(process)
			.await
			.context("Failed to run the JS process.")?
			.context("The JS process did not run successfully.")?;

		// Evaluate the resulting expression.
		let value = self
			.evaluate(expression)
			.await
			.context("Failed to evaluate expression returned by JS process.")?;

		Ok(value)
	}
}
