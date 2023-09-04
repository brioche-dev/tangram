use super::Build;
use crate::{
	error::{Error, Result},
	server::Server,
	util::task_map::TaskMap,
	value::Value,
};
use futures::FutureExt;
use std::sync::Arc;

impl Build {
	#[tracing::instrument(skip(tg), ret)]
	pub async fn output(&self, tg: &Server) -> Result<Value> {
		// Get the operations task map.
		let operations_task_map = tg
			.operations_task_map
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				Arc::new(TaskMap::new(Box::new({
					let state = Arc::downgrade(&tg.state);
					move |id| {
						let state = state.clone();
						async move {
							let state = state.upgrade().unwrap();
							let tg = Server { state };
							let operation = Self::with_block(&tg, Block::with_id(id)).await?;
							let output = operation.run_inner(&tg, None).await?;
							Ok::<_, Error>(output)
						}
						.boxed()
					}
				})))
			})
			.clone();

		// Store the operation.
		self.block().store(tg).await?;

		// Run the operation.
		let value = operations_task_map.run(self.id()).await?;

		Ok(value)
	}

	#[must_use]
	#[tracing::instrument(skip(tg), ret)]
	async fn run_inner(&self, tg: &Server, parent: Option<Operation>) -> Result<Value> {
		// If the operation has already run, then return its output.
		let output = self.try_get_output(tg).await?;
		if let Some(output) = output {
			return Ok(output);
		}

		// Evaluate the operation.
		let output = match self {
			Build::Resource(resource) => resource.download_inner(tg).await?,
			Build::Target(target) => target.build_inner(tg).await?,
			Build::Task(task) => task.run_inner(tg).await?,
		};

		// Store the output.
		output.store(tg).await?;

		// Set the output.
		self.set_output_local(tg, &output).await?;

		Ok(output)
	}
}
