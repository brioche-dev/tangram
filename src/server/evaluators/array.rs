use crate::{
	expression::Expression,
	hash::Hash,
	server::{Evaluator, Server},
};
use anyhow::Result;
use async_trait::async_trait;
use futures::future::try_join_all;
use std::sync::Arc;

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
		server: &Arc<Server>,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let array = if let Expression::Array(array) = expression {
			array
		} else {
			return Ok(None);
		};

		let output_hashes =
			try_join_all(array.iter().map(|item| server.evaluate(*item, hash))).await?;
		let output_hash = server
			.add_expression(&Expression::Array(output_hashes))
			.await?;
		Ok(Some(output_hash))
	}
}
