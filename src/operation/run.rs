use super::{Hash, Operation};
use crate::{util::task_map::TaskMap, value::Value, Instance};
use anyhow::Result;
use async_recursion::async_recursion;
use futures::FutureExt;
use std::sync::Arc;

impl Instance {
	pub async fn run(self: &Arc<Self>, operation_hash: Hash) -> Result<Value> {
		// Get the operations task map.
		let operations_task_map = self
			.operations_task_map
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				Arc::new(TaskMap::new(Box::new({
					let tg = Arc::clone(self);
					move |operation_hash| {
						let tg = Arc::clone(&tg);
						async move { tg.run_inner(operation_hash, None).await.unwrap() }.boxed()
					}
				})))
			})
			.clone();

		// Run the operation.
		let value = operations_task_map.run(operation_hash).await;

		Ok(value)
	}

	#[async_recursion]
	#[must_use]
	async fn run_inner(
		self: &Arc<Self>,
		operation_hash: Hash,
		parent_operation_hash: Option<Hash>,
	) -> Result<Value> {
		// Get the operation.
		let operation = self.get_operation_local(operation_hash)?;

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
		let output = match &operation {
			Operation::Download(download) => self.run_download(download).await?,
			Operation::Process(process) => self.run_process(process).await?,
			Operation::Call(call) => self.run_call(call).await?,
		};

		// Set the operation output.
		self.set_operation_output(operation_hash, &output)?;

		Ok(output)
	}
}
