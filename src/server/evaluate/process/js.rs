use crate::{
	expression::{self, Expression},
	server::{runtime, Server},
};
use anyhow::{Context, Result};
use std::sync::Arc;

impl Server {
	pub async fn evaluate_js_process(
		self: &Arc<Self>,
		process: &expression::JsProcess,
		root_expression_hash: expression::Hash,
	) -> Result<Expression> {
		// Create a JS runtime.
		let runtime = runtime::js::Runtime::new(self);

		// Run the process.
		let expression = runtime
			.run(process)
			.await
			.context("Failed to run the JS process.")?
			.context("The JS process did not exit successfully.")?;

		// Evaluate the expression.
		let output = self
			.evaluate(&expression, root_expression_hash)
			.await
			.context("Failed to evaluate the expression returned by the JS process.")?;

		Ok(output)
	}
}
