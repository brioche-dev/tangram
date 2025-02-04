use bytes::Bytes;
use num::ToPrimitive;
use serde_v8::Serializable;
use std::{collections::BTreeMap, sync::Arc};
use tangram_client as tg;
use tangram_error::{return_error, Error, Result, WrapErr};
use url::Url;

pub fn _to_v8<'a, T>(scope: &mut v8::HandleScope<'a>, value: &T) -> Result<v8::Local<'a, v8::Value>>
where
	T: ToV8,
{
	value.to_v8(scope)
}

pub fn from_v8<'a, T>(scope: &mut v8::HandleScope<'a>, value: v8::Local<'a, v8::Value>) -> Result<T>
where
	T: FromV8,
{
	T::from_v8(scope, value)
}

pub trait ToV8 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>>;
}

pub trait FromV8: Sized {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self>;
}

impl ToV8 for () {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::undefined(scope).into())
	}
}

impl FromV8 for () {
	fn from_v8<'a>(
		_scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		if !value.is_null_or_undefined() {
			return_error!("Expected null or undefined.");
		}
		Ok(())
	}
}

impl ToV8 for bool {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Boolean::new(scope, *self).into())
	}
}

impl FromV8 for bool {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value =
			v8::Local::<v8::Boolean>::try_from(value).wrap_err("Expected a boolean value.")?;
		let value = value.boolean_value(scope);
		Ok(value)
	}
}

impl ToV8 for u8 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for u8 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_u8()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for u16 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for u16 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_u16()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for u32 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for u32 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_u32()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for u64 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for u64 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_u64()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for i8 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for i8 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_i8()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for i16 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for i16 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_i16()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for i32 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for i32 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_i32()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for i64 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for i64 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_i64()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for f32 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, self.to_f64().wrap_err("Invalid number.")?).into())
	}
}

impl FromV8 for f32 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")?
			.to_f32()
			.wrap_err("Invalid number.")
	}
}

impl ToV8 for f64 {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::Number::new(scope, *self).into())
	}
}

impl FromV8 for f64 {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		v8::Local::<v8::Number>::try_from(value)
			.wrap_err("Expected a number.")?
			.number_value(scope)
			.wrap_err("Expected a number.")
	}
}

impl ToV8 for String {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		Ok(v8::String::new(scope, self)
			.wrap_err("Failed to create the string.")?
			.into())
	}
}

impl FromV8 for String {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		if !value.is_string() {
			return_error!("Expected a string.");
		}
		Ok(value.to_rust_string_lossy(scope))
	}
}

impl<T> ToV8 for Option<T>
where
	T: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Some(value) => value.to_v8(scope),
			None => Ok(v8::undefined(scope).into()),
		}
	}
}

impl<T> FromV8 for Option<T>
where
	T: FromV8,
{
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		if value.is_null_or_undefined() {
			Ok(None)
		} else {
			Ok(Some(from_v8(scope, value)?))
		}
	}
}

impl<T> ToV8 for Arc<T>
where
	T: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.as_ref().to_v8(scope)
	}
}

impl<T> FromV8 for Arc<T>
where
	T: FromV8,
{
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		Ok(Self::new(from_v8(scope, value)?))
	}
}

impl<T1> ToV8 for (T1,)
where
	T1: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let value = self.0.to_v8(scope)?;
		let value = v8::Array::new_with_elements(scope, &[value]);
		Ok(value.into())
	}
}

impl<T1> FromV8 for (T1,)
where
	T1: FromV8,
{
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = v8::Local::<v8::Array>::try_from(value).wrap_err("Expected an array.")?;
		let value0 = value.get_index(scope, 0).wrap_err("Expected a value.")?;
		let value0 = from_v8(scope, value0)?;
		Ok((value0,))
	}
}

impl<T1, T2> ToV8 for (T1, T2)
where
	T1: ToV8,
	T2: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let value0 = self.0.to_v8(scope)?;
		let value1 = self.1.to_v8(scope)?;
		let value = v8::Array::new_with_elements(scope, &[value0, value1]);
		Ok(value.into())
	}
}

impl<T1, T2> FromV8 for (T1, T2)
where
	T1: FromV8,
	T2: FromV8,
{
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = v8::Local::<v8::Array>::try_from(value).wrap_err("Expected an array.")?;
		let value0 = value.get_index(scope, 0).wrap_err("Expected a value.")?;
		let value1 = value.get_index(scope, 1).wrap_err("Expected a value.")?;
		let value0 = from_v8(scope, value0)?;
		let value1 = from_v8(scope, value1)?;
		Ok((value0, value1))
	}
}

impl<T> ToV8 for &[T]
where
	T: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let values = self
			.iter()
			.map(|value| value.to_v8(scope))
			.collect::<Result<Vec<_>>>()?;
		let value = v8::Array::new_with_elements(scope, &values);
		Ok(value.into())
	}
}

impl<T> ToV8 for Vec<T>
where
	T: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.as_slice().to_v8(scope)
	}
}

impl<T> FromV8 for Vec<T>
where
	T: FromV8,
{
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = v8::Local::<v8::Array>::try_from(value).wrap_err("Expected an array.")?;
		let len = value.length().to_usize().unwrap();
		let mut output = Vec::with_capacity(len);
		for i in 0..len {
			let value = value
				.get_index(scope, i.to_u32().unwrap())
				.wrap_err("Expected a value.")?;
			let value = from_v8(scope, value)?;
			output.push(value);
		}
		Ok(output)
	}
}

impl<T> ToV8 for BTreeMap<String, T>
where
	T: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let output = v8::Object::new(scope);
		for (key, value) in self {
			let key = key.to_v8(scope)?;
			let value = value.to_v8(scope)?;
			output.set(scope, key, value).unwrap();
		}
		Ok(output.into())
	}
}

impl<T> FromV8 for BTreeMap<String, T>
where
	T: FromV8,
{
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = v8::Local::<v8::Object>::try_from(value).wrap_err("Expected an object.")?;
		let args = v8::GetPropertyNamesArgsBuilder::new()
			.key_conversion(v8::KeyConversionMode::ConvertToString)
			.build();
		let property_names = value.get_own_property_names(scope, args).unwrap();
		let mut output = BTreeMap::default();
		for i in 0..property_names.length() {
			let key = property_names.get_index(scope, i).unwrap();
			let value = value.get(scope, key).unwrap();
			let key = String::from_v8(scope, key)?;
			let value = from_v8(scope, value)?;
			output.insert(key, value);
		}
		Ok(output)
	}
}

impl ToV8 for serde_json::Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		serde_v8::to_v8(scope, self).wrap_err("Failed to serialize the value.")
	}
}

impl FromV8 for serde_json::Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		serde_v8::from_v8(scope, value).wrap_err("Failed to deserialize the value.")
	}
}

impl ToV8 for toml::Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		serde_v8::to_v8(scope, self).wrap_err("Failed to serialize the value.")
	}
}

impl FromV8 for toml::Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		serde_v8::from_v8(scope, value).wrap_err("Failed to deserialize the value.")
	}
}

impl ToV8 for serde_yaml::Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		serde_v8::to_v8(scope, self).wrap_err("Failed to serialize the value.")
	}
}

impl FromV8 for serde_yaml::Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		serde_v8::from_v8(scope, value).wrap_err("Failed to deserialize the value.")
	}
}

impl ToV8 for tg::Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			tg::Value::Null(value) => value.to_v8(scope),
			tg::Value::Bool(value) => value.to_v8(scope),
			tg::Value::Number(value) => value.to_v8(scope),
			tg::Value::String(value) => value.to_v8(scope),
			tg::Value::Bytes(value) => value.to_v8(scope),
			tg::Value::Leaf(value) => value.to_v8(scope),
			tg::Value::Branch(value) => value.to_v8(scope),
			tg::Value::Directory(value) => value.to_v8(scope),
			tg::Value::File(value) => value.to_v8(scope),
			tg::Value::Symlink(value) => value.to_v8(scope),
			tg::Value::Lock(value) => value.to_v8(scope),
			tg::Value::Target(value) => value.to_v8(scope),
			tg::Value::Mutation(value) => value.to_v8(scope),
			tg::Value::Template(value) => value.to_v8(scope),
			tg::Value::Array(value) => value.to_v8(scope),
			tg::Value::Map(value) => value.to_v8(scope),
		}
	}
}

impl FromV8 for tg::Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let leaf = v8::String::new_external_onebyte_static(scope, "Leaf".as_bytes()).unwrap();
		let leaf = tg.get(scope, leaf.into()).unwrap();
		let leaf = v8::Local::<v8::Function>::try_from(leaf).unwrap();

		let branch = v8::String::new_external_onebyte_static(scope, "Branch".as_bytes()).unwrap();
		let branch = tg.get(scope, branch.into()).unwrap();
		let branch = v8::Local::<v8::Function>::try_from(branch).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let lock = v8::String::new_external_onebyte_static(scope, "Lock".as_bytes()).unwrap();
		let lock = tg.get(scope, lock.into()).unwrap();
		let lock = v8::Local::<v8::Function>::try_from(lock).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		let mutation =
			v8::String::new_external_onebyte_static(scope, "Mutation".as_bytes()).unwrap();
		let mutation = tg.get(scope, mutation.into()).unwrap();
		let mutation = v8::Local::<v8::Function>::try_from(mutation).unwrap();

		let template =
			v8::String::new_external_onebyte_static(scope, "Template".as_bytes()).unwrap();
		let template = tg.get(scope, template.into()).unwrap();
		let template = v8::Local::<v8::Function>::try_from(template).unwrap();

		if value.is_null_or_undefined() {
			Ok(tg::Value::Null(()))
		} else if value.is_boolean() {
			Ok(tg::Value::Bool(from_v8(scope, value)?))
		} else if value.is_number() {
			Ok(tg::Value::Number(from_v8(scope, value)?))
		} else if value.is_string() {
			Ok(tg::Value::String(from_v8(scope, value)?))
		} else if value.is_uint8_array() {
			Ok(tg::Value::Bytes(from_v8(scope, value)?))
		} else if value.instance_of(scope, leaf.into()).unwrap() {
			Ok(tg::Value::Leaf(from_v8(scope, value)?))
		} else if value.instance_of(scope, branch.into()).unwrap() {
			Ok(tg::Value::Branch(from_v8(scope, value)?))
		} else if value.instance_of(scope, directory.into()).unwrap() {
			Ok(tg::Value::Directory(from_v8(scope, value)?))
		} else if value.instance_of(scope, file.into()).unwrap() {
			Ok(tg::Value::File(from_v8(scope, value)?))
		} else if value.instance_of(scope, symlink.into()).unwrap() {
			Ok(tg::Value::Symlink(from_v8(scope, value)?))
		} else if value.instance_of(scope, lock.into()).unwrap() {
			Ok(tg::Value::Lock(from_v8(scope, value)?))
		} else if value.instance_of(scope, target.into()).unwrap() {
			Ok(tg::Value::Target(from_v8(scope, value)?))
		} else if value.instance_of(scope, mutation.into()).unwrap() {
			Ok(tg::Value::Mutation(from_v8(scope, value)?))
		} else if value.instance_of(scope, template.into()).unwrap() {
			Ok(tg::Value::Template(from_v8(scope, value)?))
		} else if value.is_array() {
			Ok(tg::Value::Array(from_v8(scope, value)?))
		} else if value.is_object() {
			Ok(tg::Value::Map(from_v8(scope, value)?))
		} else {
			return_error!("Invalid value.");
		}
	}
}

impl ToV8 for Bytes {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let bytes = self.to_vec();
		let len = bytes.len();
		let backing_store = v8::ArrayBuffer::new_backing_store_from_vec(bytes).make_shared();
		let array_buffer = v8::ArrayBuffer::with_backing_store(scope, &backing_store);
		let uint8_array = v8::Uint8Array::new(scope, array_buffer, 0, len).unwrap();
		Ok(uint8_array.into())
	}
}

impl FromV8 for Bytes {
	fn from_v8<'a>(
		_scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let uint8_array =
			v8::Local::<v8::Uint8Array>::try_from(value).wrap_err("Expected a Uint8Array.")?;
		let slice = unsafe {
			let ptr = uint8_array
				.data()
				.cast::<u8>()
				.add(uint8_array.byte_offset());
			let len = uint8_array.byte_length();
			std::slice::from_raw_parts(ptr, len)
		};
		let bytes = Bytes::copy_from_slice(slice);
		Ok(bytes)
	}
}

impl ToV8 for tg::Blob {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Leaf(leaf) => leaf.to_v8(scope),
			Self::Branch(branch) => branch.to_v8(scope),
		}
	}
}

impl FromV8 for tg::Blob {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let leaf = v8::String::new_external_onebyte_static(scope, "Leaf".as_bytes()).unwrap();
		let leaf = tg.get(scope, leaf.into()).unwrap();
		let leaf = v8::Local::<v8::Function>::try_from(leaf).unwrap();

		let branch = v8::String::new_external_onebyte_static(scope, "Branch".as_bytes()).unwrap();
		let branch = tg.get(scope, branch.into()).unwrap();
		let branch = v8::Local::<v8::Function>::try_from(branch).unwrap();

		let blob = if value.instance_of(scope, leaf.into()).unwrap() {
			Self::Leaf(from_v8(scope, value)?)
		} else if value.instance_of(scope, branch.into()).unwrap() {
			Self::Branch(from_v8(scope, value)?)
		} else {
			return_error!("Expected a leaf or branch.")
		};

		Ok(blob)
	}
}

impl ToV8 for tg::Leaf {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let leaf = v8::String::new_external_onebyte_static(scope, "Leaf".as_bytes()).unwrap();
		let leaf = tg.get(scope, leaf.into()).unwrap();
		let leaf = v8::Local::<v8::Function>::try_from(leaf).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = leaf.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::Leaf {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let leaf = v8::String::new_external_onebyte_static(scope, "Leaf".as_bytes()).unwrap();
		let leaf = tg.get(scope, leaf.into()).unwrap();
		let leaf = v8::Local::<v8::Function>::try_from(leaf).unwrap();

		if !value.instance_of(scope, leaf.into()).unwrap() {
			return_error!("Expected a leaf.");
		}
		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for tg::leaf::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::leaf::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::leaf::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "bytes".as_bytes()).unwrap();
		let value = self.bytes.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::leaf::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let bytes = v8::String::new_external_onebyte_static(scope, "bytes".as_bytes()).unwrap();
		let bytes = value.get(scope, bytes.into()).unwrap();
		let bytes = from_v8(scope, bytes)?;

		Ok(Self { bytes })
	}
}

impl ToV8 for tg::Branch {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let branch = v8::String::new_external_onebyte_static(scope, "Branch".as_bytes()).unwrap();
		let branch = tg.get(scope, branch.into()).unwrap();
		let branch = v8::Local::<v8::Function>::try_from(branch).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = branch.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::Branch {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let branch = v8::String::new_external_onebyte_static(scope, "Branch".as_bytes()).unwrap();
		let branch = tg.get(scope, branch.into()).unwrap();
		let branch = v8::Local::<v8::Function>::try_from(branch).unwrap();

		if !value.instance_of(scope, branch.into()).unwrap() {
			return_error!("Expected a branch.");
		}
		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for tg::branch::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::branch::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::branch::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "children".as_bytes()).unwrap();
		let value = self.children.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::branch::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let children =
			v8::String::new_external_onebyte_static(scope, "children".as_bytes()).unwrap();
		let children = value.get(scope, children.into()).unwrap();
		let children = from_v8(scope, children)?;

		Ok(Self { children })
	}
}

impl ToV8 for tg::branch::Child {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "blob".as_bytes()).unwrap();
		let value = self.blob.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "size".as_bytes()).unwrap();
		let value = self.size.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::branch::Child {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let blob = v8::String::new_external_onebyte_static(scope, "blob".as_bytes()).unwrap();
		let blob = value.get(scope, blob.into()).unwrap();
		let blob = from_v8(scope, blob)?;

		let size = v8::String::new_external_onebyte_static(scope, "size".as_bytes()).unwrap();
		let size = value.get(scope, size.into()).unwrap();
		let size = from_v8(scope, size)?;

		Ok(Self { blob, size })
	}
}

impl ToV8 for tg::Artifact {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Directory(directory) => directory.to_v8(scope),
			Self::File(file) => file.to_v8(scope),
			Self::Symlink(symlink) => symlink.to_v8(scope),
		}
	}
}

impl FromV8 for tg::Artifact {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let artifact = if value.instance_of(scope, directory.into()).unwrap() {
			Self::Directory(from_v8(scope, value)?)
		} else if value.instance_of(scope, file.into()).unwrap() {
			Self::File(from_v8(scope, value)?)
		} else if value.instance_of(scope, symlink.into()).unwrap() {
			Self::Symlink(from_v8(scope, value)?)
		} else {
			return_error!("Expected a directory, file, or symlink.")
		};

		Ok(artifact)
	}
}

impl ToV8 for tg::Directory {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = directory.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::Directory {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		if !value.instance_of(scope, directory.into()).unwrap() {
			return_error!("Expected a directory.");
		}
		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for tg::directory::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::directory::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::directory::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "entries".as_bytes()).unwrap();
		let value = self.entries.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::directory::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let entries = v8::String::new_external_onebyte_static(scope, "entries".as_bytes()).unwrap();
		let entries = value.get(scope, entries.into()).unwrap();
		let entries = from_v8(scope, entries)?;

		Ok(Self { entries })
	}
}

impl ToV8 for tg::File {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = file.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::File {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		if !value.instance_of(scope, file.into()).unwrap() {
			return_error!("Expected a file.");
		}
		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for tg::file::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::file::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::file::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "contents".as_bytes()).unwrap();
		let value = self.contents.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let value = self.executable.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "references".as_bytes()).unwrap();
		let value = self.references.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::file::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let contents =
			v8::String::new_external_onebyte_static(scope, "contents".as_bytes()).unwrap();
		let contents = value.get(scope, contents.into()).unwrap();
		let contents = from_v8(scope, contents)?;

		let executable =
			v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let executable = value.get(scope, executable.into()).unwrap();
		let executable = from_v8(scope, executable)?;

		let references =
			v8::String::new_external_onebyte_static(scope, "references".as_bytes()).unwrap();
		let references = value.get(scope, references.into()).unwrap();
		let references = from_v8(scope, references)?;

		Ok(Self {
			contents,
			executable,
			references,
		})
	}
}

impl ToV8 for tg::Symlink {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = symlink.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::Symlink {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		if !value.instance_of(scope, symlink.into()).unwrap() {
			return_error!("Expected a symlink.");
		}
		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for tg::symlink::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::symlink::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::symlink::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "artifact".as_bytes()).unwrap();
		let value = self.artifact.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "path".as_bytes()).unwrap();
		let value = self.path.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::symlink::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let artifact =
			v8::String::new_external_onebyte_static(scope, "artifact".as_bytes()).unwrap();
		let artifact = value.get(scope, artifact.into()).unwrap();
		let artifact = from_v8(scope, artifact)?;

		let path = v8::String::new_external_onebyte_static(scope, "path".as_bytes()).unwrap();
		let path = value.get(scope, path.into()).unwrap();
		let path = from_v8(scope, path)?;

		Ok(Self { artifact, path })
	}
}

impl ToV8 for tg::Lock {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let lock = v8::String::new_external_onebyte_static(scope, "Lock".as_bytes()).unwrap();
		let lock = tg.get(scope, lock.into()).unwrap();
		let lock = v8::Local::<v8::Function>::try_from(lock).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = lock.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::Lock {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let lock = v8::String::new_external_onebyte_static(scope, "Lock".as_bytes()).unwrap();
		let lock = tg.get(scope, lock.into()).unwrap();
		let lock = v8::Local::<v8::Function>::try_from(lock).unwrap();

		if !value.instance_of(scope, lock.into()).unwrap() {
			return_error!("Expected a lock.");
		}
		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for tg::lock::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::lock::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::lock::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key =
			v8::String::new_external_onebyte_static(scope, "dependencies".as_bytes()).unwrap();
		let value = self
			.dependencies
			.iter()
			.map(|(key, value)| (key.to_string(), value.clone()))
			.collect::<BTreeMap<_, _>>()
			.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::lock::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let dependencies =
			v8::String::new_external_onebyte_static(scope, "dependencies".as_bytes()).unwrap();
		let dependencies = value.get(scope, dependencies.into()).unwrap();
		let dependencies: BTreeMap<String, _> = from_v8(scope, dependencies)?;
		let dependencies = dependencies
			.into_iter()
			.map(|(key, value)| (key.parse().unwrap(), value))
			.collect();

		Ok(Self { dependencies })
	}
}

impl ToV8 for tg::lock::Entry {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let value = self.package.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "lock".as_bytes()).unwrap();
		let value = self.lock.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::lock::Entry {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let package = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let package = value.get(scope, package.into()).unwrap();
		let package = from_v8(scope, package)?;

		let lock = v8::String::new_external_onebyte_static(scope, "lock".as_bytes()).unwrap();
		let lock = value.get(scope, lock.into()).unwrap();
		let lock = from_v8(scope, lock)?;

		Ok(Self { package, lock })
	}
}

impl ToV8 for tg::Mutation {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);
		match self {
			tg::Mutation::Unset => {
				let key =
					v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
				let value = "unset".to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
			},
			tg::Mutation::Set { value: value_ } => {
				let key =
					v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
				let value = "set".to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "value".as_bytes()).unwrap();
				let value = value_.clone().to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
			},
			tg::Mutation::SetIfUnset { value: value_ } => {
				let key =
					v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
				let value = "set_if_unset".to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "value".as_bytes()).unwrap();
				let value = value_.clone().to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
			},
			tg::Mutation::ArrayAppend { values } => {
				let key =
					v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
				let value = "array_append".to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "values".as_bytes()).unwrap();
				let value = values.to_v8(scope)?;
				object.set(scope, key.into(), value);
			},
			tg::Mutation::ArrayPrepend { values } => {
				let key =
					v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
				let value = "array_prepend".to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "values".as_bytes()).unwrap();
				let value = values.to_v8(scope)?;
				object.set(scope, key.into(), value);
			},
			tg::Mutation::TemplateAppend {
				template,
				separator,
			} => {
				let key =
					v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
				let value = "template_append".to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "template".as_bytes()).unwrap();
				let value = template.to_v8(scope)?;
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "separator".as_bytes()).unwrap();
				let value = separator.to_v8(scope)?;
				object.set(scope, key.into(), value);
			},
			tg::Mutation::TemplatePrepend {
				template,
				separator,
			} => {
				let key =
					v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
				let value = "template_prepend".to_v8(scope).unwrap();
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "template".as_bytes()).unwrap();
				let value = template.to_v8(scope)?;
				object.set(scope, key.into(), value);
				let key =
					v8::String::new_external_onebyte_static(scope, "separator".as_bytes()).unwrap();
				let value = separator.to_v8(scope)?;
				object.set(scope, key.into(), value);
			},
		}

		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let mutation =
			v8::String::new_external_onebyte_static(scope, "Mutation".as_bytes()).unwrap();
		let mutation = tg.get(scope, mutation.into()).unwrap();
		let mutation = v8::Local::<v8::Function>::try_from(mutation).unwrap();

		let instance = mutation.new_instance(scope, &[object.into()]).unwrap();
		Ok(instance.into())
	}
}

impl FromV8 for tg::Mutation {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let mutation =
			v8::String::new_external_onebyte_static(scope, "Mutation".as_bytes()).unwrap();
		let mutation = tg.get(scope, mutation.into()).unwrap();
		let mutation = v8::Local::<v8::Function>::try_from(mutation).unwrap();

		if !value.instance_of(scope, mutation.into()).unwrap() {
			return_error!("Expected a mutation.");
		}
		let value = value.to_object(scope).unwrap();

		let inner = v8::String::new_external_onebyte_static(scope, "inner".as_bytes()).unwrap();
		let inner = value.get(scope, inner.into()).unwrap();
		let inner = inner.to_object(scope).unwrap();

		let kind = v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
		let kind = inner.get(scope, kind.into()).unwrap();
		let kind = String::from_v8(scope, kind)?;

		match kind.as_str() {
			"unset" => Ok(tg::Mutation::Unset),
			"set" => {
				let value_ =
					v8::String::new_external_onebyte_static(scope, "value".as_bytes()).unwrap();
				let value_ = inner.get(scope, value_.into()).unwrap();
				let value_ = from_v8(scope, value_)?;
				let value_ = Box::new(value_);
				Ok(tg::Mutation::Set { value: value_ })
			},
			"set_if_unset" => {
				let value_ =
					v8::String::new_external_onebyte_static(scope, "value".as_bytes()).unwrap();
				let value_ = inner.get(scope, value_.into()).unwrap();
				let value_ = from_v8(scope, value_)?;
				let value_ = Box::new(value_);
				Ok(tg::Mutation::SetIfUnset { value: value_ })
			},
			"array_prepend" => {
				let values =
					v8::String::new_external_onebyte_static(scope, "values".as_bytes()).unwrap();
				let values = inner.get(scope, values.into()).unwrap();
				let values = from_v8(scope, values)?;
				Ok(tg::Mutation::ArrayPrepend { values })
			},
			"array_append" => {
				let values =
					v8::String::new_external_onebyte_static(scope, "values".as_bytes()).unwrap();
				let values = inner.get(scope, values.into()).unwrap();
				let values = from_v8(scope, values)?;
				Ok(tg::Mutation::ArrayAppend { values })
			},
			"template_prepend" => {
				let template =
					v8::String::new_external_onebyte_static(scope, "template".as_bytes()).unwrap();
				let template = inner.get(scope, template.into()).unwrap();
				let template = from_v8(scope, template)?;
				let separator =
					v8::String::new_external_onebyte_static(scope, "separator".as_bytes()).unwrap();
				let separator = inner.get(scope, separator.into()).unwrap();
				let separator = from_v8(scope, separator)?;
				Ok(tg::Mutation::TemplatePrepend {
					template,
					separator,
				})
			},
			"template_append" => {
				let template =
					v8::String::new_external_onebyte_static(scope, "template".as_bytes()).unwrap();
				let template = inner.get(scope, template.into()).unwrap();
				let template = from_v8(scope, template)?;
				let separator =
					v8::String::new_external_onebyte_static(scope, "separator".as_bytes()).unwrap();
				let separator = inner.get(scope, separator.into()).unwrap();
				let separator = from_v8(scope, separator)?;
				Ok(tg::Mutation::TemplateAppend {
					template,
					separator,
				})
			},
			_ => return_error!("Invalid mutation kind."),
		}
	}
}

impl ToV8 for tg::Template {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let template =
			v8::String::new_external_onebyte_static(scope, "Template".as_bytes()).unwrap();
		let template = tg.get(scope, template.into()).unwrap();
		let template = v8::Local::<v8::Function>::try_from(template).unwrap();

		let components = self.components.to_v8(scope)?;

		let instance = template.new_instance(scope, &[components]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::Template {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let template =
			v8::String::new_external_onebyte_static(scope, "Template".as_bytes()).unwrap();
		let template = tg.get(scope, template.into()).unwrap();
		let template = v8::Local::<v8::Function>::try_from(template).unwrap();

		if !value.instance_of(scope, template.into()).unwrap() {
			return_error!("Expected a template.");
		}
		let value = value.to_object(scope).unwrap();

		let components =
			v8::String::new_external_onebyte_static(scope, "components".as_bytes()).unwrap();
		let components = value.get(scope, components.into()).unwrap();
		let components = from_v8(scope, components)?;

		Ok(Self { components })
	}
}

impl ToV8 for tg::Target {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = target.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for tg::Target {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		if !value.instance_of(scope, target.into()).unwrap() {
			return_error!("Expected a target.");
		}
		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for tg::target::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::target::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::target::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "host".as_bytes()).unwrap();
		let value = self.host.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let value = self.executable.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "lock".as_bytes()).unwrap();
		let value = self.lock.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "name".as_bytes()).unwrap();
		let value = self.name.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let value = self.env.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let value = self.args.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let value = self.checksum.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tg::target::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let host = v8::String::new_external_onebyte_static(scope, "host".as_bytes()).unwrap();
		let host = value.get(scope, host.into()).unwrap();
		let host = from_v8(scope, host)?;

		let executable =
			v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let executable = value.get(scope, executable.into()).unwrap();
		let executable = from_v8(scope, executable)?;

		let lock = v8::String::new_external_onebyte_static(scope, "lock".as_bytes()).unwrap();
		let lock = value.get(scope, lock.into()).unwrap();
		let lock = from_v8(scope, lock)?;

		let name = v8::String::new_external_onebyte_static(scope, "name".as_bytes()).unwrap();
		let name = value.get(scope, name.into()).unwrap();
		let name = from_v8(scope, name)?;

		let env = v8::String::new_external_onebyte_static(scope, "env".as_bytes()).unwrap();
		let env = value.get(scope, env.into()).unwrap();
		let env = from_v8(scope, env)?;

		let args = v8::String::new_external_onebyte_static(scope, "args".as_bytes()).unwrap();
		let args = value.get(scope, args.into()).unwrap();
		let args = from_v8(scope, args)?;

		let checksum =
			v8::String::new_external_onebyte_static(scope, "checksum".as_bytes()).unwrap();
		let checksum = value.get(scope, checksum.into()).unwrap();
		let checksum = from_v8(scope, checksum)?;

		Ok(Self {
			host,
			executable,
			lock,
			name,
			env,
			args,
			checksum,
		})
	}
}

impl ToV8 for tg::template::Component {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::String(string) => string.to_v8(scope),
			Self::Artifact(artifact) => artifact.to_v8(scope),
		}
	}
}

impl FromV8 for tg::template::Component {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let directory =
			v8::String::new_external_onebyte_static(scope, "Directory".as_bytes()).unwrap();
		let directory = tg.get(scope, directory.into()).unwrap();
		let directory = v8::Local::<v8::Function>::try_from(directory).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let component = if value.is_string() {
			Self::String(from_v8(scope, value)?)
		} else if value.instance_of(scope, directory.into()).unwrap()
			|| value.instance_of(scope, file.into()).unwrap()
			|| value.instance_of(scope, symlink.into()).unwrap()
		{
			Self::Artifact(from_v8(scope, value)?)
		} else {
			return_error!("Expected a string or artifact.")
		};

		Ok(component)
	}
}

impl ToV8 for tg::object::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::object::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let (kind, value) = match self {
			Self::Leaf(blob) => ("leaf", blob.to_v8(scope)?),
			Self::Branch(blob) => ("branch", blob.to_v8(scope)?),
			Self::Directory(directory) => ("directory", directory.to_v8(scope)?),
			Self::File(file) => ("file", file.to_v8(scope)?),
			Self::Symlink(symlink) => ("symlink", symlink.to_v8(scope)?),
			Self::Lock(lock) => ("lock", lock.to_v8(scope)?),
			Self::Target(target) => ("target", target.to_v8(scope)?),
			Self::Build(_) => unreachable!(),
		};
		let object = v8::Object::new(scope);
		let key = v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
		let kind = v8::String::new_external_onebyte_static(scope, kind.as_bytes()).unwrap();
		object.set(scope, key.into(), kind.into());
		let key = v8::String::new_external_onebyte_static(scope, "value".as_bytes()).unwrap();
		object.set(scope, key.into(), value);
		Ok(object.into())
	}
}

impl FromV8 for tg::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();
		let key = v8::String::new_external_onebyte_static(scope, "kind".as_bytes()).unwrap();
		let kind = value.get(scope, key.into()).unwrap();
		let kind = String::from_v8(scope, kind).unwrap();
		let key = v8::String::new_external_onebyte_static(scope, "value".as_bytes()).unwrap();
		let value = value.get(scope, key.into()).unwrap();
		let value = match kind.as_str() {
			"leaf" => Self::Leaf(from_v8(scope, value)?),
			"branch" => Self::Branch(from_v8(scope, value)?),
			"directory" => Self::Directory(from_v8(scope, value)?),
			"file" => Self::File(from_v8(scope, value)?),
			"symlink" => Self::Symlink(from_v8(scope, value)?),
			"lock" => Self::Lock(from_v8(scope, value)?),
			"target" => Self::Target(from_v8(scope, value)?),
			_ => unreachable!(),
		};
		Ok(value)
	}
}

impl<I, O> ToV8 for tg::object::State<I, O>
where
	I: ToV8,
	O: ToV8,
{
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "id".as_bytes()).unwrap();
		let value = self.id().to_v8(scope)?;
		object.set(scope, key.into(), value);

		if self.id().is_none() {
			let key = v8::String::new_external_onebyte_static(scope, "object".as_bytes()).unwrap();
			let value = self.object().to_v8(scope)?;
			object.set(scope, key.into(), value);
		}

		Ok(object.into())
	}
}

impl<I, O> FromV8 for tg::object::State<I, O>
where
	I: FromV8,
	O: FromV8,
{
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let id = v8::String::new_external_onebyte_static(scope, "id".as_bytes()).unwrap();
		let id = value.get(scope, id.into()).unwrap();
		let id = from_v8::<Option<I>>(scope, id)?;

		let object = if id.is_none() {
			let object =
				v8::String::new_external_onebyte_static(scope, "object".as_bytes()).unwrap();
			let object = value.get(scope, object.into()).unwrap();
			from_v8(scope, object)?
		} else {
			None
		};

		Ok(Self::new(id, object))
	}
}

impl ToV8 for tg::blob::ArchiveFormat {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::blob::ArchiveFormat {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::blob::CompressionFormat {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::blob::CompressionFormat {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::Checksum {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::Checksum {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::checksum::Algorithm {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::checksum::Algorithm {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for tg::System {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for tg::System {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for Error {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let error = v8::String::new_external_onebyte_static(scope, "Error".as_bytes()).unwrap();
		let error = tg.get(scope, error.into()).unwrap();
		let error = v8::Local::<v8::Function>::try_from(error).unwrap();

		let message = self.message.to_v8(scope)?;
		let location = self.location.to_v8(scope)?;
		let stack = self.stack.to_v8(scope)?;
		let source = self.source.to_v8(scope)?;

		let instance = error
			.new_instance(scope, &[message, location, stack, source])
			.unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for Error {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let error = v8::String::new_external_onebyte_static(scope, "Error".as_bytes()).unwrap();
		let error = tg.get(scope, error.into()).unwrap();
		let error = v8::Local::<v8::Function>::try_from(error).unwrap();

		if !value.instance_of(scope, error.into()).unwrap() {
			return_error!("Expected an error.");
		}
		let value = value.to_object(scope).unwrap();

		let message = v8::String::new_external_onebyte_static(scope, "message".as_bytes()).unwrap();
		let message = value.get(scope, message.into()).unwrap();
		let message = from_v8(scope, message)?;

		let location =
			v8::String::new_external_onebyte_static(scope, "location".as_bytes()).unwrap();
		let location = value.get(scope, location.into()).unwrap();
		let location = from_v8(scope, location)?;

		let stack = v8::String::new_external_onebyte_static(scope, "stack".as_bytes()).unwrap();
		let stack = value.get(scope, stack.into()).unwrap();
		let stack = from_v8(scope, stack)?;

		let source = v8::String::new_external_onebyte_static(scope, "source".as_bytes()).unwrap();
		let source = value.get(scope, source.into()).unwrap();
		let source = from_v8::<Option<Error>>(scope, source)?.map(|error| Arc::new(error) as _);

		Ok(Error {
			message,
			location,
			stack,
			source,
		})
	}
}

impl ToV8 for tangram_error::Location {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "source".as_bytes()).unwrap();
		let value = self.source.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "line".as_bytes()).unwrap();
		let value = self.line.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "column".as_bytes()).unwrap();
		let value = self.column.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for tangram_error::Location {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let source = v8::String::new_external_onebyte_static(scope, "source".as_bytes()).unwrap();
		let source = value.get(scope, source.into()).unwrap();
		let source = from_v8(scope, source)?;

		let line = v8::String::new_external_onebyte_static(scope, "line".as_bytes()).unwrap();
		let line = value.get(scope, line.into()).unwrap();
		let line = from_v8(scope, line)?;

		let column = v8::String::new_external_onebyte_static(scope, "column".as_bytes()).unwrap();
		let column = value.get(scope, column.into()).unwrap();
		let column = from_v8(scope, column)?;

		Ok(Self {
			source,
			line,
			column,
		})
	}
}

impl ToV8 for Url {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for Url {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?
			.parse()
			.wrap_err("Failed to parse the string as a URL.")
	}
}
