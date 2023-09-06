use crate::{artifact, template, Client, Result};

crate::id!();

crate::kind!(Symlink);

#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

#[derive(Clone, Debug)]
pub struct Value {
	pub target: template::Handle,
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let target = template::Handle::with_id(data.target);
		Value { target }
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		todo!()
	}
}

#[derive(Clone, Debug, tangram_serialize::Deserialize, tangram_serialize::Serialize)]
pub struct Data {
	#[tangram_serialize(id = 0)]
	pub target: crate::template::Id,
}

impl Handle {
	#[must_use]
	pub fn new(target: template::Handle) -> Self {
		Self::with_value(Value { target })
	}

	pub async fn target(&self, tg: &Client) -> Result<template::Handle> {
		Ok(self.value(tg).await?.target.clone())
	}

	pub async fn resolve(&self, tg: &Client) -> Result<Option<artifact::Handle>> {
		self.resolve_from(tg, None).await
	}

	#[allow(clippy::unused_async)]
	pub async fn resolve_from(
		&self,
		_tg: &Client,
		_from: Option<&Value>,
	) -> Result<Option<artifact::Handle>> {
		unimplemented!()
	}
}

impl Value {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Handle> {
		vec![self.target.clone().into()]
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		vec![self.target.into()]
	}
}
