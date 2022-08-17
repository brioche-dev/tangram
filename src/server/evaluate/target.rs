use crate::{
	expression::{self, Expression},
	server::Server,
};
use anyhow::Result;
use async_recursion::async_recursion;
use camino::Utf8PathBuf;
use std::sync::Arc;

impl Server {
	#[allow(clippy::must_use_candidate)]
	#[async_recursion]
	pub async fn evaluate_target(
		self: &Arc<Self>,
		target: expression::Target,
	) -> Result<Expression> {
		let expression =
			expression::Expression::Process(expression::Process::Js(expression::JsProcess {
				lockfile: target.lockfile,
				module: Box::new(expression::Expression::Path(expression::Path {
					artifact: Box::new(expression::Expression::Artifact(target.package)),
					path: Some(Utf8PathBuf::from("tangram.js")),
				})),
				export: target.name,
				args: target.args,
			}));

		// TODO Check my cache.

		// TODO Ask the peers.

		Ok(expression)
	}
}
