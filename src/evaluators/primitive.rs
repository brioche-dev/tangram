use crate::{builder, evaluators::Evaluator, expression::Expression, hash::Hash};
use anyhow::Result;
use async_trait::async_trait;

pub struct Primitive;

impl Primitive {
	#[must_use]
	pub fn new() -> Primitive {
		Primitive {}
	}
}

impl Default for Primitive {
	fn default() -> Self {
		Primitive::new()
	}
}

#[async_trait]
impl Evaluator for Primitive {
	async fn evaluate(
		&self,
		_builder: &builder::Shared,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>> {
		match &expression {
			Expression::Null
			| Expression::Bool(_)
			| Expression::Number(_)
			| Expression::String(_)
			| Expression::Artifact(_)
			| Expression::Directory(_)
			| Expression::File(_)
			| Expression::Symlink(_)
			| Expression::Dependency(_) => Ok(Some(hash)),
			_ => Ok(None),
		}
	}
}
