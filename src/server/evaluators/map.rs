use crate::{
	expression::Expression,
	hash::Hash,
	server::{Evaluator, Server},
};
use anyhow::Result;
use async_trait::async_trait;
use futures::{future::try_join_all, TryFutureExt};
use std::sync::Arc;

pub struct Map;

impl Map {
	#[must_use]
	pub fn new() -> Map {
		Map {}
	}
}

impl Default for Map {
	fn default() -> Self {
		Map::new()
	}
}

#[async_trait]
impl Evaluator for Map {
	async fn evaluate(
		&self,
		server: &Arc<Server>,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let map = if let Expression::Map(map) = expression {
			map
		} else {
			return Ok(None);
		};
		let outputs = try_join_all(map.iter().map(|(key, value)| {
			server
				.evaluate(*value, hash)
				.map_ok(|value| (Arc::clone(key), value))
		}))
		.await?
		.into_iter()
		.collect();
		let output = Expression::Map(outputs);
		let output_hash = server.add_expression(&output).await?;
		Ok(Some(output_hash))
	}
}
