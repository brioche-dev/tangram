#[cfg(feature = "v8")]
use crate::{
	error::Result,
	target::{from_v8, FromV8, ToV8},
};
#[cfg(not(feature = "v8"))]
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Bytes(
	#[cfg(not(feature = "v8"))] Arc<[u8]>,
	#[cfg(feature = "v8")] crate::target::Buffer,
);

impl Bytes {
	#[must_use]
	pub fn with_boxed_slice(bytes: Box<[u8]>) -> Self {
		Self(bytes.into())
	}

	#[must_use]
	pub fn with_vec(bytes: Vec<u8>) -> Self {
		Self(bytes.into())
	}

	#[must_use]
	pub fn as_slice(&self) -> &[u8] {
		&self.0
	}
}

impl AsRef<[u8]> for Bytes {
	fn as_ref(&self) -> &[u8] {
		self.as_slice()
	}
}

impl From<Box<[u8]>> for Bytes {
	fn from(bytes: Box<[u8]>) -> Self {
		Self::with_boxed_slice(bytes)
	}
}

impl From<Vec<u8>> for Bytes {
	fn from(bytes: Vec<u8>) -> Self {
		Self::with_vec(bytes)
	}
}

impl serde::Serialize for Bytes {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		todo!()
	}
}

impl<'de> serde::Deserialize<'de> for Bytes {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		todo!()
	}
}

impl tangram_serialize::Serialize for Bytes {
	fn serialize<W>(&self, serializer: &mut tangram_serialize::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		todo!()
	}
}

impl tangram_serialize::Deserialize for Bytes {
	fn deserialize<R>(
		deserializer: &mut tangram_serialize::Deserializer<R>,
	) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		todo!()
	}
}

#[cfg(feature = "v8")]
impl ToV8 for Bytes {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.0.to_v8(scope)
	}
}

#[cfg(feature = "v8")]
impl FromV8 for Bytes {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		Ok(Self(from_v8(scope, value)?))
	}
}
