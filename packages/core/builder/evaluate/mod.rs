use super::Shared;
use crate::{expression::Expression, hash::Hash};
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use either::Either;
use futures::FutureExt;
use std::future::Future;

pub mod array;
pub mod fetch;
pub mod js;
pub mod map;
pub mod package;
pub mod process;
pub mod target;
pub mod template;

impl Shared {
	/// Evaluate an [`Expression`].
	#[async_recursion]
	#[must_use]
	pub async fn evaluate(&self, hash: Hash, parent_hash: Hash) -> Result<Hash> {
		// Add the evaluation.
		self.add_evaluation(parent_hash, hash)?;

		// Get the expression and the output hash if the expression was previously evaluated.
		let (expression, output_hash) = self.get_expression_local_with_output(hash)?;

		// If the expression was previously evaluated, return the output hash.
		if let Some(output_hash) = output_hash {
			// Return the output hash.
			return Ok(output_hash);
		}

		// Evaluate the expression.
		let output_hash = match &expression {
			Expression::Null
			| Expression::Bool(_)
			| Expression::Number(_)
			| Expression::String(_)
			| Expression::Artifact(_)
			| Expression::Directory(_)
			| Expression::File(_)
			| Expression::Symlink(_)
			| Expression::Dependency(_) => futures::future::ok(hash).boxed(),
			Expression::Template(template) => self.evaluate_template(hash, template).boxed(),
			Expression::Package(package) => self.evaluate_package(hash, package).boxed(),
			Expression::Js(js) => self.evaluate_js(hash, js).boxed(),
			Expression::Fetch(fetch) => self
				.evaluate_or_await_in_progress_evaluation(hash, || self.evaluate_fetch(hash, fetch))
				.boxed(),
			Expression::Process(process) => self
				.evaluate_or_await_in_progress_evaluation(hash, || {
					self.evaluate_process(hash, process)
				})
				.boxed(),
			Expression::Target(target) => self.evaluate_target(hash, target).boxed(),
			Expression::Array(array) => self.evaluate_array(hash, array).boxed(),
			Expression::Map(map) => self.evaluate_map(hash, map).boxed(),
		}
		.await?;

		// Set the expression output.
		self.set_expression_output(hash, output_hash)?;

		Ok(output_hash)
	}

	async fn evaluate_or_await_in_progress_evaluation<F, Fut>(
		&self,
		hash: Hash,
		evaluate: F,
	) -> Result<Hash>
	where
		F: FnOnce() -> Fut,
		Fut: Future<Output = Result<Hash>>,
	{
		// Determine if we should await an in progress evaluation or perform the evaluation ourselves.
		let receiver_or_sender = {
			let mut in_progress_evaluations = self.in_progress_evaluations.lock().unwrap();
			if let Some(receiver) = in_progress_evaluations.get(&hash) {
				let receiver = receiver.resubscribe();
				Either::Left(receiver)
			} else {
				let (sender, receiver) = tokio::sync::broadcast::channel(1);
				in_progress_evaluations.insert(hash, receiver);
				Either::Right(sender)
			}
		};

		match receiver_or_sender {
			// Await an in progress evaluation.
			Either::Left(mut receiver) => {
				let output_hash = receiver.recv().await.context(
					"Failed to receive the hash of the in progress evaluation on the receiver.",
				)?;
				Ok(output_hash)
			},

			// Perform the evaluation and send the output hash to the broadcast channel.
			Either::Right(sender) => {
				let output_hash = evaluate().await?;
				let mut in_progress_evaluations = self.in_progress_evaluations.lock().unwrap();
				sender
					.send(output_hash)
					.context("Failed to send the output hash on the broadcast channel.")?;
				in_progress_evaluations.remove(&hash);
				Ok(output_hash)
			},
		}
	}
}
