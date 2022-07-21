use crate::expression::{self, Expression};
use crate::server::Server;
use anyhow::Result;
use async_recursion::async_recursion;
use std::sync::Arc;

impl Server {
	#[allow(clippy::must_use_candidate)]
	#[async_recursion]
	pub async fn evaluate_target(
		self: &Arc<Self>,
		_target: expression::Target,
	) -> Result<Expression> {
		let _build_lock_guard = self.lock.read().await;
		todo!()
	}
}
