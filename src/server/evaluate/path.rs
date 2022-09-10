use crate::{
	expression::{self, Expression},
	server::Server,
};
use anyhow::Result;
use std::sync::Arc;

impl Server {
	pub async fn evaluate_path(self: &Arc<Self>, path: &expression::Path) -> Result<Expression> {
		let artifact = self.evaluate(&path.artifact).await?;
		let output = Expression::Path(crate::expression::Path {
			artifact: Box::new(artifact),
			path: path.path.clone(),
		});
		Ok(output)
	}
}
