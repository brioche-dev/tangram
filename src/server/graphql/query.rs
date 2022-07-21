use super::scalars::{ExpressionJson, ValueJson};
use crate::server::Server;
use std::sync::Arc;

pub struct Query;

#[juniper::graphql_object(context = Arc<Server>)]
impl Query {
	async fn evaluate(
		server: &Arc<Server>,
		expression: ExpressionJson,
	) -> juniper::FieldResult<ValueJson> {
		let expression = expression.try_into()?;
		let value = server.evaluate(expression).await?;
		let value = value.try_into()?;
		Ok(value)
	}
}
