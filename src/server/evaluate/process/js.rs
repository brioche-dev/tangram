use crate::{
	expression,
	hash::Hash,
	server::{runtime, Server},
};
use anyhow::{Context, Result};
use std::sync::Arc;

impl Server {
	pub async fn evaluate_js_process(
		self: &Arc<Self>,
		hash: Hash,
		process: &expression::JsProcess,
	) -> Result<Hash> {
		// Create a JS runtime.
		let runtime = runtime::js::Runtime::new(self);

		// Run the process.
		let output_hash = runtime
			.run(process)
			.await
			.context("Failed to run the JS process.")?
			.context("The JS process did not exit successfully.")?;

		// Evaluate the expression.
		let output = self
			.evaluate(output_hash, hash)
			.await
			.context("Failed to evaluate the expression returned by the JS process.")?;

		Ok(output)
	}
}
