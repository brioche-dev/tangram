use crate::{
	expression::{self, Expression},
	server::Server,
};
use anyhow::Result;
use std::sync::Arc;

impl Server {
	pub async fn evaluate_path(
		self: &Arc<Self>,
		path: &expression::Path,
		root_expression_hash: expression::Hash,
	) -> Result<Expression> {
		// Evaluate the artifact.
		let artifact = self.evaluate(&path.artifact, root_expression_hash).await?;
		let output = Expression::Path(crate::expression::Path {
			artifact: Box::new(artifact),
			path: path.path.clone(),
		});
		Ok(output)
	}
}
