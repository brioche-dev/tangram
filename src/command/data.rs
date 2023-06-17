use super::Command;
use crate::{
	checksum::Checksum,
	error::Result,
	instance::Instance,
	operation,
	system::System,
	template::{self, Template},
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
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub system: System,

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

	#[tangram_serialize(id = 7)]
	#[serde(default)]
	pub host_paths: Vec<String>,
}

impl Command {
	pub fn to_data(&self) -> Data {
		let system = self.system;
		let command = self.executable.to_data();
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
		let host_paths = self.host_paths.clone();
		Data {
			system,
			executable: command,
			env,
			args,
			checksum,
			unsafe_,
			network,
			host_paths,
		}
	}

	pub async fn from_data(tg: &Instance, hash: operation::Hash, data: Data) -> Result<Self> {
		let system = data.system;
		let command = Template::from_data(tg, data.executable).await?;
		let env = try_join_all(data.env.into_iter().map(|(key, value)| async move {
			let value = Template::from_data(tg, value).await?;
			Ok::<_, crate::error::Error>((key, value))
		}))
		.await?
		.into_iter()
		.collect();
		let args = try_join_all(
			data.args
				.into_iter()
				.map(|arg| Template::from_data(tg, arg)),
		)
		.await?;
		let checksum = data.checksum;
		let unsafe_ = data.unsafe_;
		let network = data.network;
		let host_paths = data.host_paths;
		Ok(Self {
			hash,
			system,
			executable: command,
			env,
			args,
			checksum,
			unsafe_,
			network,
			host_paths,
		})
	}
}
