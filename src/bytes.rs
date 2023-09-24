#[cfg(feature = "server")]
use crate::Result;
use std::ops::Range;
#[cfg(not(feature = "server"))]
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Bytes {
	buffer: Buffer,
	range: Range<usize>,
}

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
				formatter.write_str("a string or byte buf")
			}
			fn visit_byte_buf<E>(self, v: Vec<u8>) -> std::result::Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				Ok(Bytes::with_buffer(Buffer::from(v.into_boxed_slice())))
			}

			fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
			where
				E: serde::de::Error,
			{
				Ok(Bytes::with_buffer(Buffer::from(
					hex::decode(v).map_err(E::custom)?,
				)))
			}

			fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
			where
				A: serde::de::SeqAccess<'de>,
			{
				let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
				while let Some(byte) = seq.next_element()? {
					bytes.push(byte);
				}
				Ok(Bytes::with_buffer(Buffer::from(bytes.into_boxed_slice())))
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
	#[cfg(not(feature = "server"))] Arc<[u8]>,
	#[cfg(feature = "server")] v8::SharedRef<v8::BackingStore>,
);

impl Buffer {
	#[cfg(feature = "server")]
	#[must_use]
	pub fn backing_store(&self) -> &v8::SharedRef<v8::BackingStore> {
		&self.0
	}
}

#[cfg(feature = "server")]
impl From<v8::SharedRef<v8::BackingStore>> for Buffer {
	fn from(value: v8::SharedRef<v8::BackingStore>) -> Self {
		Self(value)
	}
}

impl AsRef<[u8]> for Buffer {
	fn as_ref(&self) -> &[u8] {
		#[cfg(not(feature = "server"))]
		{
			&self.0
		}
		#[cfg(feature = "server")]
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
			#[cfg(not(feature = "server"))]
			value.into(),
			#[cfg(feature = "server")]
			v8::ArrayBuffer::new_backing_store_from_boxed_slice(value).make_shared(),
		)
	}
}

impl From<Vec<u8>> for Buffer {
	fn from(value: Vec<u8>) -> Self {
		Self(
			#[cfg(not(feature = "server"))]
			value.into(),
			#[cfg(feature = "server")]
			v8::ArrayBuffer::new_backing_store_from_vec(value).make_shared(),
		)
	}
}
