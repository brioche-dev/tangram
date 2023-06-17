use super::{Function, Kind};
use crate::{
	artifact,
	error::Result,
	instance::Instance,
	operation,
	path::Subpath,
	value::{self, Value},
};
use futures::future::try_join_all;
use std::collections::BTreeMap;

#[derive(
	Clone,
	Debug,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub package_hash: artifact::Hash,

	#[tangram_serialize(id = 1)]
	pub module_path: Subpath,

	#[tangram_serialize(id = 2)]
	pub kind: Kind,

	#[tangram_serialize(id = 3)]
	pub name: String,

	#[tangram_serialize(id = 4)]
	pub env: BTreeMap<String, value::Data>,

	#[tangram_serialize(id = 5)]
	pub args: Vec<value::Data>,
}

impl Function {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let env = self
			.env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args = self.args.iter().map(Value::to_data).collect();
		Data {
			package_hash: self.package_hash,
			module_path: self.module_path.clone(),
			kind: self.kind,
			name: self.name.clone(),
			env,
			args,
		}
	}

	pub async fn from_data(tg: &Instance, hash: operation::Hash, data: Data) -> Result<Self> {
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
			package_hash: data.package_hash,
			module_path: data.module_path,
			name: data.name,
			kind: data.kind,
			env,
			args,
		})
	}
}
