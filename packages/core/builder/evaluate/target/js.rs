use crate::{builder::State, expression::Target, hash::Hash, js};
use anyhow::{Context, Result};

impl State {
	pub async fn evaluate_target_js(&self, hash: Hash, target: &Target) -> Result<Hash> {
		// Get a handle to the current tokio runtime.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create a channel to receive the output.
		let (sender, receiver) = tokio::sync::oneshot::channel();

		// Clone the builder and the process expression to send to the thread.
		let builder = self.builder();
		let target = target.clone();

		// Run the js runtime on its own thread.
		let thread = std::thread::spawn(move || {
			// Create a tokio runtime for the current thread.
			let runtime = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.unwrap();

			// Run the JS process.
			let result = runtime.block_on(async move {
				let mut runtime = js::Runtime::new(builder, main_runtime_handle).await?;
				let output_hash = runtime.run(hash, &target).await?;
				Ok::<_, anyhow::Error>(output_hash)
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
