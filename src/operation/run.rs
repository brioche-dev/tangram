use super::{Operation, OperationHash};
use crate::{value::Value, State};
use anyhow::Result;
use async_recursion::async_recursion;

impl State {
	pub async fn run(&self, operation: &Operation) -> Result<Value> {
		self.run_inner(operation, None).await
	}

	#[async_recursion]
	#[must_use]
	pub async fn run_inner(
		&self,
		operation: &Operation,
		parent_operation_hash: Option<OperationHash>,
	) -> Result<Value> {
		// Get the operation hash.
		let operation_hash = operation.hash();

		// Add the run.
		if let Some(parent_operation_hash) = parent_operation_hash {
			self.add_operation_child(parent_operation_hash, operation_hash)?;
		}

		// Get the operation output.
		let output = self.get_output(operation_hash)?;

		// If the operation has already run, then return its output.
		if let Some(output) = output {
			// Return the output.
			return Ok(output);
		}

		// Run the operation.
		let output = match operation {
			Operation::Download(download) => self.run_download(download).await?,
			Operation::Process(process) => self.run_process(process).await?,
			Operation::Target(target) => self.run_target(target).await?,
		};

		// Set the output.
		self.set_output(operation_hash, &output)?;

		Ok(output)
	}
}
