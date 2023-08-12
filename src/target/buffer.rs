use crate::error::{Error, Result, WrapErr};
use crate::target::{FromV8, ToV8};
use std::ops::Range;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct Buffer {
	backing_store: v8::SharedRef<v8::BackingStore>,
	range: Range<usize>,
}

impl Buffer {
	#[must_use]
	pub fn new(bytes: Box<[u8]>, range: Range<usize>) -> Self {
		let backing_store =
			v8::ArrayBuffer::new_backing_store_from_boxed_slice(bytes).make_shared();
		Self {
			backing_store,
			range,
		}
	}

	#[must_use]
	pub fn with_vec(bytes: Vec<u8>) -> Self {
		let range = 0..bytes.len();
		let bytes = bytes.into_boxed_slice();
		Self::new(bytes, range)
	}

	#[must_use]
	pub fn with_boxed_slice(bytes: Box<[u8]>) -> Self {
		let range = 0..bytes.len();
		Self::new(bytes, range)
	}

	#[must_use]
	pub fn as_slice(&self) -> &[u8] {
		unsafe {
			std::slice::from_raw_parts(
				self.backing_store
					.data()
					.unwrap()
					.as_ptr()
					.cast::<u8>()
					.add(self.range.start),
				self.range.len(),
			)
		}
	}
}

impl std::ops::Deref for Buffer {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		self.as_slice()
	}
}

impl AsRef<[u8]> for Buffer {
	fn as_ref(&self) -> &[u8] {
		self.as_slice()
	}
}

impl From<Box<[u8]>> for Buffer {
	fn from(value: Box<[u8]>) -> Self {
		Self::with_boxed_slice(value)
	}
}

impl From<Vec<u8>> for Buffer {
	fn from(value: Vec<u8>) -> Self {
		Self::with_vec(value)
	}
}

impl ToV8 for Buffer {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let value = v8::ArrayBuffer::with_backing_store(scope, &self.backing_store);
		let value = v8::Uint8Array::new(scope, value, self.range.start, self.range.len()).unwrap();
		Ok(value.into())
	}
}

impl FromV8 for Buffer {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = v8::Local::<v8::Uint8Array>::try_from(value)
			.map_err(Error::other)
			.wrap_err("Expected a Uint8Array.")?;
		let backing_store = value
			.buffer(scope)
			.wrap_err("Expected the Uint8Array to have a buffer.")?
			.get_backing_store();
		let range = value.byte_offset()..(value.byte_offset() + value.byte_length());
		Ok(Self {
			backing_store,
			range,
		})
	}
}
