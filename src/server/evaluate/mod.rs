use crate::{artifact::Artifact, expression::Expression, hash::Hash, server::Server, value::Value};
use anyhow::Result;
use async_recursion::async_recursion;
use futures::TryFutureExt;
use sqlx::prelude::*;
use std::sync::Arc;

mod fetch;
mod path;
mod process;
mod target;
mod template;

impl Server {
	/// Retrieve the memoized value from a previous evaluation of an expression, if one exists.
	pub(crate) async fn _get_memoized_value_for_expression(
		&self,
		expression: &Expression,
	) -> Result<Option<Value>> {
		let expression_json = serde_json::to_vec(&expression)?;
		let expression_hash = Hash::new(&expression_json);
		let row = sqlx::query(
			r#"
				select value
				from expressions
				where expression_hash = $1
			"#,
		)
		.bind(&expression_hash.to_string())
		.fetch_optional(&self._database_pool)
		.await?;
		let value = if let Some(row) = row {
			let value_json: String = row.get_unchecked(0);
			let value = serde_json::from_str(&value_json)?;
			Some(value)
		} else {
			None
		};
		Ok(value)
	}

	/// Memoize the value from the evaluation of an expression.
	pub(crate) async fn _set_memoized_value_for_expression(
		&self,
		expression: &Expression,
		value: &Value,
	) -> Result<()> {
		let expression_json = serde_json::to_vec(&expression)?;
		let expression_hash = Hash::new(&expression_json);
		let value_json = serde_json::to_vec(&value)?;
		sqlx::query(
			r#"
				insert into expressions (
					expression_hash, expression, value
				) values (
					$1, $2, $3
				)
			"#,
		)
		.bind(&expression_hash.to_string())
		.bind(&expression_json)
		.bind(&value_json)
		.execute(&self._database_pool)
		.await?;
		Ok(())
	}

	/// Evaluate an [`Expression`].
	#[async_recursion]
	#[must_use]
	pub async fn evaluate(self: &Arc<Self>, expression: Expression) -> Result<Value> {
		// Acquire the build lock.
		let _build_lock_guard = self.lock.read().await;

		// Recursively evaluate the expression.
		let value = match expression {
			Expression::Null => Value::Null,
			Expression::Bool(value) => Value::Bool(value),
			Expression::Number(value) => Value::Number(value),
			Expression::String(value) => Value::String(value),
			Expression::Artifact(artifact) => Value::Artifact(Artifact(artifact.0)),
			Expression::Path(path) => self.evaluate_path(path).await?,
			Expression::Template(template) => self.evaluate_template(template).await?,
			Expression::Fetch(fetch) => self.evaluate_fetch(fetch).await?,
			Expression::Process(process) => self.evaluate_process(process).await?,
			Expression::Target(target) => {
				self.evaluate(self.evaluate_target(target).await?).await?
			},
			Expression::Array(value) => {
				let values = value.into_iter().map(|value| self.evaluate(value));
				let array = futures::future::try_join_all(values).await?;
				Value::Array(array)
			},
			Expression::Map(value) => {
				let values = value.into_iter().map(|(key, expression)| {
					self.evaluate(expression).map_ok(|value| (key, value))
				});
				let value = futures::future::try_join_all(values).await?;
				let map = value.into_iter().collect();
				Value::Map(map)
			},
		};

		Ok(value)
	}
}
