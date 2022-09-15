use crate::{
	expression::{self, Expression},
	hash::Hash,
	server::Server,
};
use anyhow::Result;
use std::sync::Arc;

impl Server {
	pub async fn evaluate_path(
		self: &Arc<Self>,
		path: &expression::Path,
		parent_hash: Hash,
	) -> Result<Hash> {
		// Evaluate the artifact.
		let artifact = self.evaluate(path.artifact, parent_hash).await?;
		let output = Expression::Path(crate::expression::Path {
			artifact,
			path: path.path.clone(),
		});
		let output_hash = self.add_expression(&output).await?;
		Ok(output_hash)
	}
}
