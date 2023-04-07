use crate::{
	error::Result,
	function::{self, Function},
	instance::Instance,
	operation,
	value::{self, Value},
};
use futures::future::try_join_all;
use std::collections::BTreeMap;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Data {
	#[buffalo(id = 0)]
	pub function: function::Data,

	#[buffalo(id = 1)]
	pub env: BTreeMap<String, value::Data>,

	#[buffalo(id = 2)]
	pub args: Vec<value::Data>,
}

impl super::Call {
	pub fn to_data(&self) -> Data {
		let function = self.function.to_data();
		let env = self
			.env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args = self.args.iter().map(Value::to_data).collect();
		Data {
			function,
			env,
			args,
		}
	}

	pub async fn from_data(tg: &Instance, hash: operation::Hash, data: Data) -> Result<Self> {
		let function = Function::from_data(data.function);
		let env = try_join_all(data.env.into_iter().map(|(key, value)| async move {
			Ok::<_, crate::error::Error>((key, Value::from_data(tg, value).await?))
		}))
		.await?
		.into_iter()
		.collect();
		let args = try_join_all(
			data.args
				.into_iter()
				.map(|value| async move { Value::from_data(tg, value).await }),
		)
		.await?;
		Ok(Self {
			hash,
			function,
			env,
			args,
		})
	}
}
