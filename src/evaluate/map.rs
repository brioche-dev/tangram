use crate::{
	expression::{Expression, Map},
	hash::Hash,
	State,
};
use anyhow::Result;
use futures::{future::try_join_all, TryFutureExt};
use std::sync::Arc;

impl State {
	pub(super) async fn evaluate_map(&self, hash: Hash, map: &Map) -> Result<Hash> {
		let outputs = try_join_all(map.iter().map(|(key, value)| {
			self.evaluate(*value, hash)
				.map_ok(|value| (Arc::clone(key), value))
		}))
		.await?
		.into_iter()
		.collect();
		let output = Expression::Map(outputs);
		let output_hash = self.add_expression(&output).await?;
		Ok(output_hash)
	}
}
