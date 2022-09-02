use crate::{
	expression::Expression,
	expression::{self},
	server::Server,
	value::Value,
};
use anyhow::Result;
use async_recursion::async_recursion;
use camino::Utf8PathBuf;
use std::sync::Arc;

impl Server {
	#[allow(clippy::must_use_candidate)]
	#[async_recursion]
	pub async fn evaluate_target(self: &Arc<Self>, target: expression::Target) -> Result<Value> {
		// Return a memoized value if one is available.
		let expression = Expression::Target(target);
		let value = self.get_memoized_value_for_expression(&expression).await?;

		if let Some(value) = value {
			return Ok(value);
		};

		// Evaluate.
		let target = match expression {
			Expression::Target(target) => target,
			_ => unreachable!(),
		};
		let expression =
			expression::Expression::Process(expression::Process::Js(expression::JsProcess {
				lockfile: target.lockfile.clone(),
				module: Box::new(expression::Expression::Path(expression::Path {
					artifact: Box::new(expression::Expression::Artifact(target.package.clone())),
					path: Some(Utf8PathBuf::from("tangram.js")),
				})),
				export: target.name.clone(),
				args: target.args.clone(),
			}));
		let value = self.evaluate(expression).await?;

		// Memoize the value.
		self.set_memoized_value_for_expression(&Expression::Target(target), &value)
			.await?;

		Ok(value)
	}
}
