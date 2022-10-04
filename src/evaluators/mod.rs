use crate::{builder, expression::Expression, hash::Hash};
use anyhow::Result;
use async_trait::async_trait;

pub mod array;
pub mod fetch;
pub mod js;
pub mod map;
pub mod package;
pub mod primitive;
pub mod process;
pub mod target;
pub mod template;

#[async_trait]
pub trait Evaluator {
	async fn evaluate(
		&self,
		builder: &builder::Shared,
		hash: Hash,
		expression: &Expression,
	) -> Result<Option<Hash>>;
}
