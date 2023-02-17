use super::{Hash, Operation};
use crate::{value::Value, Cli};
use anyhow::Result;
use async_recursion::async_recursion;
use std::sync::Arc;

impl Cli {
	pub async fn run(self: &Arc<Self>, operation: &Operation) -> Result<Value> {
		self.run_with_parent(operation, None).await
	}

	#[async_recursion]
	#[must_use]
	async fn run_with_parent(
		self: &Arc<Self>,
		operation: &Operation,
		parent_operation_hash: Option<Hash>,
	) -> Result<Value> {
		// Get the operation hash.
		let operation_hash = operation.hash();

		// Add the operation child.
		if let Some(parent_operation_hash) = parent_operation_hash {
			self.add_operation_child(parent_operation_hash, operation_hash)?;
		}

		// Attempt to get the operation output.
		let output = self.get_operation_output(operation_hash)?;

		// If the operation has already run, then return its output.
		if let Some(output) = output {
			return Ok(output);
		}

		// Run the operation.
		let output = match operation {
			Operation::Download(download) => self.run_download(download).await?,
			Operation::Process(process) => self.run_process(process).await?,
			Operation::Call(call) => self.run_call(call).await?,
		};

		// Set the operation output.
		self.set_operation_output(operation_hash, &output)?;

		Ok(output)
	}
}
