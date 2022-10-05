use crate::{builder, evaluators::Evaluator, expression::Expression, hash::Hash};
use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;

pub struct Array;

impl Array {
	#[must_use]
	pub fn new() -> Array {
		Array {}
	}
}

impl Default for Array {
	fn default() -> Self {
		Array::new()
	}
}

#[async_trait]
impl Evaluator for Array {
	async fn evaluate(
		&self,
		builder: &builder::Shared,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let array = if let Expression::Array(array) = expression {
			array
		} else {
			return Ok(None);
		};

		let output_hashes =
			try_join_all(array.iter().map(|item| builder.evaluate(*item, hash))).await?;
		let output_hash = builder
			.add_expression(&Expression::Array(output_hashes))
			.await?;
		Ok(Some(output_hash))
	}
}
