use super::Operation;
use crate::{
	error::{Error, Result},
	instance::Instance,
	util::task_map::TaskMap,
	value::Value,
};
use futures::FutureExt;
use std::sync::Arc;

impl Operation {
	#[tracing::instrument(skip(tg), ret)]
	pub async fn run(&self, tg: &Arc<Instance>) -> Result<Value> {
		// Get the operations task map.
		let operations_task_map = tg
			.operations_task_map
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				Arc::new(TaskMap::new(Box::new({
					let tg = Arc::downgrade(tg);
					move |operation_hash| {
						let tg = tg.clone();
						async move {
							let tg = tg.upgrade().unwrap();
							let operation = Self::get(&tg, operation_hash).await?;
							let output = operation.run_inner(&tg, None).await?;
							Ok::<_, Error>(output)
						}
						.boxed()
					}
				})))
			})
			.clone();

		// Run the operation.
		let value = operations_task_map.run(self.hash()).await?;

		Ok(value)
	}

	#[must_use]
	#[tracing::instrument(skip(tg), ret)]
	async fn run_inner(&self, tg: &Arc<Instance>, parent: Option<Operation>) -> Result<Value> {
		// Add this operation as a child of its parent.
		if let Some(parent) = parent {
			self.add_child(tg, &parent).await?;
		}

		// Attempt to get the operation output. If the operation has already run, then return its output.
		let output = self.output(tg).await?;
		if let Some(output) = output {
			return Ok(output);
		}

		// Run the operation.
		let output = match self {
			Operation::Command(command) => command.run_inner(tg).await?,
			Operation::Function(function) => function.call_inner(tg).await?,
			Operation::Resource(resource) => resource.download_inner(tg).await?,
		};

		// Set the operation output.
		self.set_output(tg, &output).await?;

		Ok(output)
	}
}
