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
	pub async fn evaluate(&self, tg: &Instance, parent: Option<Operation>) -> Result<Value> {
		// Get the operations task map.
		let operations_task_map = tg
			.operations_task_map
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				Arc::new(TaskMap::new(Box::new({
					let state = Arc::downgrade(&tg.state);
					move |block| {
						let state = state.clone();
						async move {
							let state = state.upgrade().unwrap();
							let tg = Instance { state };
							let operation = Self::get(&tg, block).await?;
							let output = operation.run_inner(&tg, None).await?;
							Ok::<_, Error>(output)
						}
						.boxed()
					}
				})))
			})
			.clone();

		// Run the operation.
		let value = operations_task_map.run(self.block()).await?;

		Ok(value)
	}

	#[must_use]
	#[tracing::instrument(skip(tg), ret)]
	async fn run_inner(&self, tg: &Instance, parent: Option<Operation>) -> Result<Value> {
		// If the operation has already run, then return its output value.
		let output = self.try_get_output(tg).await?;
		if let Some(output) = output {
			return Ok(output);
		}

		// Evaluate the operation.
		let output = match self {
			Operation::Resource(resource) => resource.download_inner(tg).await?,
			Operation::Target(target) => target.build_inner(tg).await?,
			Operation::Task(task) => task.run_inner(tg).await?,
		};

		// Set the operation output value.
		self.set_output_local(tg, &output).await?;

		Ok(output)
	}
}
