use crate::{
	expression::{Array, Expression},
	hash::Hash,
	State,
};
use anyhow::Result;
use futures::future::try_join_all;

impl State {
	pub(super) async fn evaluate_array(&self, hash: Hash, array: &Array) -> Result<Hash> {
		let output_hashes =
			try_join_all(array.iter().map(|item| self.evaluate(*item, hash))).await?;
		let output_hash = self
			.add_expression(&Expression::Array(output_hashes))
			.await?;
		Ok(output_hash)
	}
}
