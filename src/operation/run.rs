use super::{Hash, Operation};
use crate::{error::Result, util::task_map::TaskMap, value::Value, Instance};
use async_recursion::async_recursion;
use futures::FutureExt;
use std::sync::{Arc, Weak};

impl Operation {
	pub async fn run(&self, tg: &Arc<Instance>) -> crate::Result<Value> {
		let operation_hash = tg.add_operation(self)?;
		let value = tg.run_operation(operation_hash).await?;
		Ok(value)
	}
}

impl Instance {
	#[tracing::instrument(skip(self), ret)]
	pub async fn run_operation(self: &Arc<Self>, operation_hash: Hash) -> Result<Value> {
		// Get the operations task map.
		let operations_task_map = self
			.operations_task_map
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				Arc::new(TaskMap::new(Box::new({
					let tg = Arc::downgrade(self);
					move |operation_hash| {
						let tg = Weak::clone(&tg);
						async move {
							let tg = Weak::upgrade(&tg).unwrap();
							tg.run_operation_inner(operation_hash, None).await
						}
						.boxed()
					}
				})))
			})
			.clone();

		// Run the operation.
		let value = operations_task_map.run(operation_hash).await?;

		Ok(value)
	}

	#[async_recursion]
	#[must_use]
	#[tracing::instrument(skip(self), ret)]
	async fn run_operation_inner(
		self: Arc<Self>,
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
		let output = self.get_operation_output_local(operation_hash)?;

		// If the operation has already run, then return its output.
		if let Some(output) = output {
			tracing::debug!("Operation already ran.");
			return Ok(output);
		}

		tracing::debug!("Running operation.");

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
