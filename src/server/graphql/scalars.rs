use crate::{expression::Expression, repl::Repl, value::Value};
use juniper::GraphQLScalarValue;

#[derive(GraphQLScalarValue, serde::Serialize, serde::Deserialize)]
pub struct ExpressionJson(pub String);

impl TryFrom<Expression> for ExpressionJson {
	type Error = anyhow::Error;
	fn try_from(value: Expression) -> Result<Self, Self::Error> {
		let expression = serde_json::to_string(&value)?;
		Ok(ExpressionJson(expression))
	}
}

impl TryFrom<ExpressionJson> for Expression {
	type Error = anyhow::Error;
	fn try_from(value: ExpressionJson) -> Result<Self, Self::Error> {
		let value = serde_json::from_str(&value.0)?;
		Ok(value)
	}
}

#[derive(GraphQLScalarValue, serde::Serialize, serde::Deserialize)]
pub struct ValueJson(pub String);

impl TryFrom<Value> for ValueJson {
	type Error = anyhow::Error;
	fn try_from(value: Value) -> Result<Self, Self::Error> {
		let repl = serde_json::to_string(&value)?;
		Ok(ValueJson(repl))
	}
}

impl TryFrom<ValueJson> for Value {
	type Error = anyhow::Error;
	fn try_from(value: ValueJson) -> Result<Self, Self::Error> {
		let value = serde_json::from_str(&value.0)?;
		Ok(value)
	}
}

#[derive(GraphQLScalarValue, serde::Serialize, serde::Deserialize)]
pub struct ReplJson(pub String);

impl TryFrom<Repl> for ReplJson {
	type Error = anyhow::Error;
	fn try_from(value: Repl) -> Result<Self, Self::Error> {
		let repl = serde_json::to_string(&value)?;
		Ok(ReplJson(repl))
	}
}

impl TryFrom<ReplJson> for Repl {
	type Error = anyhow::Error;
	fn try_from(value: ReplJson) -> Result<Self, Self::Error> {
		let value = serde_json::from_str(&value.0)?;
		Ok(value)
	}
}
