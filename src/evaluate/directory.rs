use crate::{
	expression::{self, Expression},
	hash::Hash,
	State,
};
use anyhow::Result;
use futures::future::try_join_all;
use std::collections::BTreeMap;

impl State {
	pub(super) async fn evaluate_directory(
		&self,
		hash: Hash,
		directory: &expression::Directory,
	) -> Result<Hash> {
		// Evaluate the directory entries.
		let entries = directory.entries.iter().map(|(name, entry)| async {
			Ok::<_, anyhow::Error>((name.clone(), self.evaluate(*entry, hash).await?))
		});
		let entries: BTreeMap<String, Hash> = try_join_all(entries).await?.into_iter().collect();

		// Create the output.
		let output = Expression::Directory(expression::Directory { entries });
		let output_hash = self.add_expression(&output).await?;

		Ok(output_hash)
	}
}
