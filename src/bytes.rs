#[cfg(feature = "build")]
use crate::error::{Error, Result, WrapErr};
use std::ops::Range;
#[cfg(not(feature = "build"))]
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Bytes {
	buffer: Buffer,
	range: Range<usize>,
}

crate::value!(Bytes);

impl Bytes {
	#[must_use]
	pub fn empty() -> Self {
		Self::with_buffer(vec![].into())
	}

	#[must_use]
	pub fn with_slice(slice: &[u8]) -> Self {
		Self::with_buffer(slice.to_owned().into())
	}

	#[must_use]
	pub fn with_boxed_slice(slice: Box<[u8]>) -> Self {
		Self::with_buffer(slice.into())
	}

	#[must_use]
	pub fn with_vec(vec: Vec<u8>) -> Self {
		Self::with_buffer(vec.into())
	}

	#[must_use]
	pub fn with_buffer(buffer: Buffer) -> Self {
		let range = 0..buffer.as_ref().len();
		Self::new(buffer, range)
	}

	#[must_use]
	pub fn new(buffer: Buffer, range: Range<usize>) -> Self {
		Self { buffer, range }
	}

	#[must_use]
	pub fn buffer(&self) -> &Buffer {
		&self.buffer
	}

	#[must_use]
	pub fn range(&self) -> Range<usize> {
		self.range.clone()
	}

	#[must_use]
	pub fn as_slice(&self) -> &[u8] {
		&self.buffer.as_ref()[self.range.clone()]
	}
}

impl AsRef<[u8]> for Bytes {
	fn as_ref(&self) -> &[u8] {
		self.as_slice()
	}
}

impl std::ops::Deref for Bytes {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		self.as_slice()
	}
}

impl serde::Serialize for Bytes {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_bytes(self.as_slice())
	}
}

impl<'de> serde::Deserialize<'de> for Bytes {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Visitor;
		impl<'de> serde::de::Visitor<'de> for Visitor {
			type Value = Bytes;
			fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
				formatter.write_str("a byte buf")
			}
			fn visit_byte_buf<E>(self, v: Vec<u8>) -> std::result::Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				Ok(Bytes::with_buffer(Buffer::from(v.into_boxed_slice())))
			}
		}
		deserializer.deserialize_byte_buf(Visitor)
	}
}

impl tangram_serialize::Serialize for Bytes {
	fn serialize<W>(&self, serializer: &mut tangram_serialize::Serializer<W>) -> std::io::Result<()>
	where
		W: std::io::Write,
	{
		serializer.serialize_bytes(self.as_slice())
	}
}

impl tangram_serialize::Deserialize for Bytes {
	fn deserialize<R>(
		deserializer: &mut tangram_serialize::Deserializer<R>,
	) -> std::io::Result<Self>
	where
		R: std::io::Read,
	{
		let bytes = deserializer.deserialize_bytes()?;
		Ok(Bytes::with_vec(bytes))
	}
}

#[derive(Clone, Debug)]
pub struct Buffer(
	#[cfg(not(feature = "build"))] Arc<[u8]>,
	#[cfg(feature = "build")] v8::SharedRef<v8::BackingStore>,
);

impl AsRef<[u8]> for Buffer {
	fn as_ref(&self) -> &[u8] {
		#[cfg(not(feature = "build"))]
		{
			&self.0
		}
		#[cfg(feature = "build")]
		unsafe {
			std::slice::from_raw_parts(
				self.0.data().unwrap().as_ptr().cast::<u8>(),
				self.0.byte_length(),
			)
		}
	}
}

impl From<Box<[u8]>> for Buffer {
	fn from(value: Box<[u8]>) -> Self {
		Self(
			#[cfg(not(feature = "build"))]
			value.into(),
			#[cfg(feature = "build")]
			unsafe {
				v8::ArrayBuffer::new_backing_store_from_boxed_slice(value).make_shared()
			},
		)
	}
}

impl From<Vec<u8>> for Buffer {
	fn from(value: Vec<u8>) -> Self {
		Self(
			#[cfg(not(feature = "build"))]
			value.into(),
			#[cfg(feature = "build")]
			unsafe {
				v8::ArrayBuffer::new_backing_store_from_vec(value).make_shared()
			},
		)
	}
}
