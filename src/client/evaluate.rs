use super::Client;
use crate::{
	expression::Expression,
	server::graphql::{ExpressionJson, ValueJson},
	value::Value,
};
use anyhow::Result;
use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
	schema_path = "./src/server/graphql/schema.graphql",
	query_path = "./src/client/evaluate.graphql"
)]
struct EvaluateQuery;

impl Client {
	pub async fn evaluate(&self, expression: Expression) -> Result<Value> {
		let expression = expression.try_into()?;
		let value = self
			.request::<EvaluateQuery>(evaluate_query::Variables { expression })
			.await?
			.evaluate;
		let value = value.try_into()?;
		Ok(value)
	}
}
