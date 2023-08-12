use super::Task;
use crate::{
	block::Block,
	checksum::Checksum,
	error::Result,
	instance::Instance,
	system::System,
	template::{self, Template},
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
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub host: System,

	#[tangram_serialize(id = 1)]
	pub executable: template::Data,

	#[tangram_serialize(id = 2)]
	#[serde(default)]
	pub env: BTreeMap<String, template::Data>,

	#[tangram_serialize(id = 3)]
	#[serde(default)]
	pub args: Vec<template::Data>,

	#[tangram_serialize(id = 4)]
	#[serde(default)]
	pub checksum: Option<Checksum>,

	#[tangram_serialize(id = 5)]
	#[serde(default, rename = "unsafe")]
	pub unsafe_: bool,

	#[tangram_serialize(id = 6)]
	#[serde(default)]
	pub network: bool,
}

impl Task {
	pub fn to_data(&self) -> Data {
		let host = self.host;
		let executable = self.executable.to_data();
		let env = self
			.env
			.iter()
			.map(|(key, value)| {
				let key = key.clone();
				let value = value.to_data();
				(key, value)
			})
			.collect();
		let args = self.args.iter().map(Template::to_data).collect();
		let checksum = self.checksum.clone();
		let unsafe_ = self.unsafe_;
		let network = self.network;
		Data {
			host,
			executable,
			env,
			args,
			checksum,
			unsafe_,
			network,
		}
	}

	pub async fn from_data(tg: &Instance, block: Block, data: Data) -> Result<Self> {
		let host = data.host;
		let executable = Template::from_data(tg, data.executable).await?;
		let env = data
			.env
			.into_iter()
			.map(|(key, value)| async move {
				let value = Template::from_data(tg, value).await?;
				Ok::<_, crate::error::Error>((key, value))
			})
			.collect::<FuturesUnordered<_>>()
			.try_collect()
			.await?;
		let args = data
			.args
			.into_iter()
			.map(|arg| Template::from_data(tg, arg))
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		let checksum = data.checksum;
		let unsafe_ = data.unsafe_;
		let network = data.network;
		Ok(Self {
			block,
			host,
			executable,
			env,
			args,
			checksum,
			unsafe_,
			network,
		})
	}
}
