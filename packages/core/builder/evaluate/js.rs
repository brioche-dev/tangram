use crate::{builder::Shared, expression::Js, hash::Hash, js};
use anyhow::{Context, Result};

impl Shared {
	pub(super) async fn evaluate_js(&self, hash: Hash, js: &Js) -> Result<Hash> {
		// Get a handle to the current tokio runtime.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create a channel to receive the output.
		let (sender, receiver) = tokio::sync::oneshot::channel();

		// Clone the builder and js expression to send to the thread.
		let builder = self.clone();
		let js = js.clone();

		// Run the js runtime on its own thread.
		let thread = std::thread::spawn(|| {
			// Create a single threaded tokio runtime.
			let rt = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.context("Failed to create the runtime.")?;

			// Run the JS process.
			let result = rt.block_on(async move {
				let mut runtime = js::Runtime::new(builder, main_runtime_handle).await?;
				let hash = runtime.js(&js).await?;
				Ok::<_, anyhow::Error>(hash)
			});

			// Notify the receiver that the process is complete.
			sender.send(()).unwrap();

			result
		});

		// Wait for the thread to complete.
		receiver.await.unwrap();

		// Join the thread to receive the output.
		let output_hash = thread
			.join()
			.unwrap()
			.context("There was an error in the JS process.")?;

		// Evaluate the expression.
		let output_hash = self
			.evaluate(output_hash, hash)
			.await
			.context("Failed to evaluate the expression returned by the JS process.")?;

		Ok(output_hash)
	}
}
