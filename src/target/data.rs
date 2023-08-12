use super::Target;
use crate::{
	block::Block,
	error::Result,
	id::Id,
	instance::Instance,
	path::Subpath,
	value::{self, Value},
};
use futures::{
	stream::{FuturesOrdered, FuturesUnordered},
	TryStreamExt,
};
use std::collections::BTreeMap;

#[derive(
	Clone,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub package: Id,

	#[tangram_serialize(id = 1)]
	pub path: Subpath,

	#[tangram_serialize(id = 2)]
	pub name: String,

	#[tangram_serialize(id = 3)]
	pub env: BTreeMap<String, value::Data>,

	#[tangram_serialize(id = 4)]
	pub args: Vec<value::Data>,
}

impl Target {
	#[must_use]
	pub fn to_data(&self) -> Data {
		let env = self
			.env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args = self.args.iter().map(Value::to_data).collect();
		Data {
			package: self.package.id(),
			path: self.path.clone(),
			name: self.name.clone(),
			env,
			args,
		}
	}

	pub async fn from_data(tg: &Instance, block: Block, data: Data) -> Result<Self> {
		let package = Block::with_id(data.package);
		let module_path = data.path;
		let name = data.name;
		let env = data
			.env
			.into_iter()
			.map(|(key, value)| async move {
				Ok::<_, crate::error::Error>((key, Value::from_data(tg, value).await?))
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;
		let args = data
			.args
			.into_iter()
			.map(|value| async move { Value::from_data(tg, value).await })
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		Ok(Self {
			block,
			package,
			path: module_path,
			name,
			env,
			args,
		})
	}
}
