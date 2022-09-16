use crate::{
	expression::Expression,
	hash::Hash,
	server::{Evaluator, Server},
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

pub struct Path;

impl Path {
	#[must_use]
	pub fn new() -> Path {
		Path {}
	}
}

impl Default for Path {
	fn default() -> Self {
		Path::new()
	}
}

#[async_trait]
impl Evaluator for Path {
	async fn evaluate(
		&self,
		server: &Arc<Server>,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		let path = if let Expression::Path(path) = expression {
			path
		} else {
			return Ok(None);
		};
		let artifact = server.evaluate(path.artifact, hash).await?;
		let output = Expression::Path(crate::expression::Path {
			artifact,
			path: path.path.clone(),
		});
		let output_hash = server.add_expression(&output).await?;
		Ok(Some(output_hash))
	}
}
