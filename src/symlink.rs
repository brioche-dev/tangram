use crate::{artifact, template, value, Client, Result};

crate::id!(Symlink);

#[derive(Clone, Debug)]
pub struct Handle(value::Handle);

crate::handle!(Symlink);

#[derive(Clone, Debug)]
pub struct Value {
	pub target: template::Handle,
}

crate::value!(Symlink);

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
	pub target: crate::template::Id,
}

impl Handle {
	#[must_use]
	pub fn new(target: template::Handle) -> Self {
		Self::with_value(Value { target })
	}

	pub async fn target(&self, client: &Client) -> Result<template::Handle> {
		Ok(self.value(client).await?.target.clone())
	}

	pub async fn resolve(&self, client: &Client) -> Result<Option<artifact::Handle>> {
		self.resolve_from(client, None).await
	}

	#[allow(clippy::unused_async)]
	pub async fn resolve_from(
		&self,
		_client: &Client,
		_from: Option<&Value>,
	) -> Result<Option<artifact::Handle>> {
		unimplemented!()
	}
}

impl Value {
	#[allow(clippy::needless_pass_by_value)]
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		let target = template::Handle::with_id(data.target);
		Self { target }
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			target: self.target.expect_id(),
		}
	}

	#[must_use]
	pub fn children(&self) -> Vec<value::Handle> {
		vec![self.target.clone().into()]
	}
}

impl Data {
	#[must_use]
	pub fn children(&self) -> Vec<crate::Id> {
		vec![self.target.into()]
	}
}
