use bytes::Bytes;
use num::ToPrimitive;
use std::{collections::BTreeMap, sync::Arc};
use tangram_client as tg;
use tg::{
	blob, checksum, directory, error, file,
	object::{self, Object},
	package, return_error, symlink, target, template, Artifact, Blob, Checksum, Directory, Error,
	File, Package, Relpath, Result, Subpath, Symlink, System, Target, Template, Value, WrapErr,
};
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
		for (key, value) in self.iter() {
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
		let property_names = value
			.get_own_property_names(scope, v8::GetPropertyNamesArgs::default())
			.unwrap();
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

impl ToV8 for Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Value::Null(value) => value.to_v8(scope),
			Value::Bool(value) => value.to_v8(scope),
			Value::Number(value) => value.to_v8(scope),
			Value::String(value) => value.to_v8(scope),
			Value::Bytes(value) => value.to_v8(scope),
			Value::Blob(value) => value.to_v8(scope),
			Value::Directory(value) => value.to_v8(scope),
			Value::File(value) => value.to_v8(scope),
			Value::Symlink(value) => value.to_v8(scope),
			Value::Template(value) => value.to_v8(scope),
			Value::Package(value) => value.to_v8(scope),
			Value::Target(value) => value.to_v8(scope),
			Value::Array(value) => value.to_v8(scope),
			Value::Map(value) => value.to_v8(scope),
		}
	}
}

impl FromV8 for Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let blob = v8::String::new_external_onebyte_static(scope, "Blob".as_bytes()).unwrap();
		let blob = tg.get(scope, blob.into()).unwrap();
		let blob = v8::Local::<v8::Function>::try_from(blob).unwrap();

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

		let template =
			v8::String::new_external_onebyte_static(scope, "Template".as_bytes()).unwrap();
		let template = tg.get(scope, template.into()).unwrap();
		let template = v8::Local::<v8::Function>::try_from(template).unwrap();

		let package = v8::String::new_external_onebyte_static(scope, "Package".as_bytes()).unwrap();
		let package = tg.get(scope, package.into()).unwrap();
		let package = v8::Local::<v8::Function>::try_from(package).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		if value.is_null_or_undefined() {
			Ok(Value::Null(()))
		} else if value.is_boolean() {
			Ok(Value::Bool(from_v8(scope, value)?))
		} else if value.is_number() {
			Ok(Value::Number(from_v8(scope, value)?))
		} else if value.is_string() {
			Ok(Value::String(from_v8(scope, value)?))
		} else if value.is_uint8_array() {
			Ok(Value::Bytes(from_v8(scope, value)?))
		} else if value.instance_of(scope, blob.into()).unwrap() {
			Ok(Value::Blob(from_v8(scope, value)?))
		} else if value.instance_of(scope, directory.into()).unwrap() {
			Ok(Value::Directory(from_v8(scope, value)?))
		} else if value.instance_of(scope, file.into()).unwrap() {
			Ok(Value::File(from_v8(scope, value)?))
		} else if value.instance_of(scope, symlink.into()).unwrap() {
			Ok(Value::Symlink(from_v8(scope, value)?))
		} else if value.instance_of(scope, template.into()).unwrap() {
			Ok(Value::Template(from_v8(scope, value)?))
		} else if value.instance_of(scope, package.into()).unwrap() {
			Ok(Value::Package(from_v8(scope, value)?))
		} else if value.instance_of(scope, target.into()).unwrap() {
			Ok(Value::Target(from_v8(scope, value)?))
		} else if value.is_array() {
			Ok(Value::Array(from_v8(scope, value)?))
		} else if value.is_object() {
			Ok(Value::Map(from_v8(scope, value)?))
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

impl ToV8 for Relpath {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for Relpath {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for Subpath {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for Subpath {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for Blob {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let blob = v8::String::new_external_onebyte_static(scope, "Blob".as_bytes()).unwrap();
		let blob = tg.get(scope, blob.into()).unwrap();
		let blob = v8::Local::<v8::Function>::try_from(blob).unwrap();

		let handle = self.handle().to_v8(scope)?;

		let instance = blob.new_instance(scope, &[handle]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for Blob {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let blob = v8::String::new_external_onebyte_static(scope, "Blob".as_bytes()).unwrap();
		let blob = tg.get(scope, blob.into()).unwrap();
		let blob = v8::Local::<v8::Function>::try_from(blob).unwrap();

		if !value.instance_of(scope, blob.into()).unwrap() {
			return_error!("Expected a blob.");
		}
		let value = value.to_object(scope).unwrap();

		let handle = v8::String::new_external_onebyte_static(scope, "handle".as_bytes()).unwrap();
		let handle = value.get(scope, handle.into()).unwrap();
		let handle = from_v8(scope, handle)?;

		Ok(Self::with_handle(handle))
	}
}

impl ToV8 for blob::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			blob::Object::Branch(children) => Ok(children.to_v8(scope)?),
			blob::Object::Leaf(bytes) => Ok(bytes.to_v8(scope)?),
		}
	}
}

impl FromV8 for blob::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		if value.is_array() {
			Ok(blob::Object::Branch(from_v8(scope, value)?))
		} else if value.is_uint8_array() {
			Ok(blob::Object::Leaf(from_v8(scope, value)?))
		} else {
			return_error!("Invalid blob object.");
		}
	}
}

impl ToV8 for Artifact {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::Directory(directory) => directory.to_v8(scope),
			Self::File(file) => file.to_v8(scope),
			Self::Symlink(symlink) => symlink.to_v8(scope),
		}
	}
}

impl FromV8 for Artifact {
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

impl ToV8 for Directory {
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

		let handle = self.handle().to_v8(scope)?;

		let instance = directory.new_instance(scope, &[handle]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for Directory {
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

		let handle = v8::String::new_external_onebyte_static(scope, "handle".as_bytes()).unwrap();
		let handle = value.get(scope, handle.into()).unwrap();
		let handle = from_v8(scope, handle)?;

		Ok(Self::with_handle(handle))
	}
}

impl ToV8 for directory::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "entries".as_bytes()).unwrap();
		let value = self.entries.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for directory::Object {
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

impl ToV8 for File {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let file = v8::String::new_external_onebyte_static(scope, "File".as_bytes()).unwrap();
		let file = tg.get(scope, file.into()).unwrap();
		let file = v8::Local::<v8::Function>::try_from(file).unwrap();

		let handle = self.handle().to_v8(scope)?;

		let instance = file.new_instance(scope, &[handle]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for File {
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

		let handle = v8::String::new_external_onebyte_static(scope, "handle".as_bytes()).unwrap();
		let handle = value.get(scope, handle.into()).unwrap();
		let handle = from_v8(scope, handle)?;

		Ok(Self::with_handle(handle))
	}
}

impl ToV8 for file::Object {
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

impl FromV8 for file::Object {
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

impl ToV8 for Symlink {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let symlink = v8::String::new_external_onebyte_static(scope, "Symlink".as_bytes()).unwrap();
		let symlink = tg.get(scope, symlink.into()).unwrap();
		let symlink = v8::Local::<v8::Function>::try_from(symlink).unwrap();

		let handle = self.handle().to_v8(scope)?;

		let instance = symlink.new_instance(scope, &[handle]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for Symlink {
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

		let handle = v8::String::new_external_onebyte_static(scope, "handle".as_bytes()).unwrap();
		let handle = value.get(scope, handle.into()).unwrap();
		let handle = from_v8(scope, handle)?;

		Ok(Self::with_handle(handle))
	}
}

impl ToV8 for symlink::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "target".as_bytes()).unwrap();
		let value = self.target.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for symlink::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "target".as_bytes()).unwrap();
		let target = value.get(scope, target.into()).unwrap();
		let target = from_v8(scope, target)?;

		Ok(Self { target })
	}
}

impl ToV8 for Template {
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

impl FromV8 for Template {
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

impl ToV8 for template::Component {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		match self {
			Self::String(string) => string.to_v8(scope),
			Self::Artifact(artifact) => artifact.to_v8(scope),
		}
	}
}

impl FromV8 for template::Component {
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

impl ToV8 for Package {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let package = v8::String::new_external_onebyte_static(scope, "Package".as_bytes()).unwrap();
		let package = tg.get(scope, package.into()).unwrap();
		let package = v8::Local::<v8::Function>::try_from(package).unwrap();

		let handle = self.handle().to_v8(scope)?;

		let instance = package.new_instance(scope, &[handle]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for Package {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let package = v8::String::new_external_onebyte_static(scope, "Package".as_bytes()).unwrap();
		let package = tg.get(scope, package.into()).unwrap();
		let package = v8::Local::<v8::Function>::try_from(package).unwrap();

		if !value.instance_of(scope, package.into()).unwrap() {
			return_error!("Expected a package.");
		}
		let value = value.to_object(scope).unwrap();

		let handle = v8::String::new_external_onebyte_static(scope, "handle".as_bytes()).unwrap();
		let handle = value.get(scope, handle.into()).unwrap();
		let handle = from_v8(scope, handle)?;

		Ok(Self::with_handle(handle))
	}
}

impl ToV8 for package::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "artifact".as_bytes()).unwrap();
		let value = self.artifact.to_v8(scope)?;
		object.set(scope, key.into(), value);

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

impl FromV8 for package::Object {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let artifact =
			v8::String::new_external_onebyte_static(scope, "artifact".as_bytes()).unwrap();
		let artifact = value.get(scope, artifact.into()).unwrap();
		let artifact = from_v8(scope, artifact)?;

		let dependencies =
			v8::String::new_external_onebyte_static(scope, "dependencies".as_bytes()).unwrap();
		let dependencies = value.get(scope, dependencies.into()).unwrap();
		let dependencies: BTreeMap<String, _> = from_v8(scope, dependencies)?;
		let dependencies = dependencies
			.into_iter()
			.map(|(key, value)| (key.parse().unwrap(), value))
			.collect();

		Ok(Self {
			artifact,
			dependencies,
		})
	}
}

impl ToV8 for Target {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let target = v8::String::new_external_onebyte_static(scope, "Target".as_bytes()).unwrap();
		let target = tg.get(scope, target.into()).unwrap();
		let target = v8::Local::<v8::Function>::try_from(target).unwrap();

		let handle = self.handle().to_v8(scope)?;

		let instance = target.new_instance(scope, &[handle]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for Target {
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

		let handle = v8::String::new_external_onebyte_static(scope, "handle".as_bytes()).unwrap();
		let handle = value.get(scope, handle.into()).unwrap();
		let handle = from_v8(scope, handle)?;

		Ok(Self::with_handle(handle))
	}
}

impl ToV8 for target::Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "host".as_bytes()).unwrap();
		let value = self.host.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "executable".as_bytes()).unwrap();
		let value = self.executable.to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let value = self.package.to_v8(scope)?;
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

		let key = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let value = self.unsafe_.to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for target::Object {
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

		let package = v8::String::new_external_onebyte_static(scope, "package".as_bytes()).unwrap();
		let package = value.get(scope, package.into()).unwrap();
		let package = from_v8(scope, package)?;

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

		let unsafe_ = v8::String::new_external_onebyte_static(scope, "unsafe".as_bytes()).unwrap();
		let unsafe_ = value.get(scope, unsafe_.into()).unwrap();
		let unsafe_ = from_v8(scope, unsafe_)?;

		Ok(Self {
			host,
			executable,
			package,
			name,
			env,
			args,
			checksum,
			unsafe_,
		})
	}
}

impl ToV8 for object::Id {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for object::Id {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for object::Handle {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let object_ = v8::String::new_external_onebyte_static(scope, "Object_".as_bytes()).unwrap();
		let object_ = tg.get(scope, object_.into()).unwrap();
		let object_ = object_.to_object(scope).unwrap();

		let handle = v8::String::new_external_onebyte_static(scope, "Handle".as_bytes()).unwrap();
		let handle = object_.get(scope, handle.into()).unwrap();
		let handle = v8::Local::<v8::Function>::try_from(handle).unwrap();

		let state = self.state().read().unwrap().to_v8(scope)?;

		let instance = handle.new_instance(scope, &[state]).unwrap();

		Ok(instance.into())
	}
}

impl FromV8 for object::Handle {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let context = scope.get_current_context();
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		let object_ = v8::String::new_external_onebyte_static(scope, "Object_".as_bytes()).unwrap();
		let object_ = tg.get(scope, object_.into()).unwrap();
		let object_ = object_.to_object(scope).unwrap();

		let handle = v8::String::new_external_onebyte_static(scope, "Handle".as_bytes()).unwrap();
		let handle = object_.get(scope, handle.into()).unwrap();
		let handle = v8::Local::<v8::Function>::try_from(handle).unwrap();

		if !value.instance_of(scope, handle.into()).unwrap() {
			return_error!("Expected a handle.");
		}

		let value = value.to_object(scope).unwrap();

		let state = v8::String::new_external_onebyte_static(scope, "state".as_bytes()).unwrap();
		let state = value.get(scope, state.into()).unwrap();
		let state = from_v8(scope, state)?;

		Ok(Self::with_state(state))
	}
}

impl ToV8 for object::State {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let object = v8::Object::new(scope);

		let key = v8::String::new_external_onebyte_static(scope, "id".as_bytes()).unwrap();
		let value = self.id().to_v8(scope)?;
		object.set(scope, key.into(), value);

		let key = v8::String::new_external_onebyte_static(scope, "object".as_bytes()).unwrap();
		let value = self.object().to_v8(scope)?;
		object.set(scope, key.into(), value);

		Ok(object.into())
	}
}

impl FromV8 for object::State {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		let value = value.to_object(scope).unwrap();

		let id = v8::String::new_external_onebyte_static(scope, "id".as_bytes()).unwrap();
		let id = value.get(scope, id.into()).unwrap();
		let id = from_v8(scope, id)?;

		let object = v8::String::new_external_onebyte_static(scope, "object".as_bytes()).unwrap();
		let object = value.get(scope, object.into()).unwrap();
		let object = from_v8(scope, object)?;

		Ok(Self::new(id, object))
	}
}

impl ToV8 for Object {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		let (kind, value) = match self {
			Self::Blob(blob) => ("blob", blob.to_v8(scope)?),
			Self::Directory(directory) => ("directory", directory.to_v8(scope)?),
			Self::File(file) => ("file", file.to_v8(scope)?),
			Self::Symlink(symlink) => ("symlink", symlink.to_v8(scope)?),
			Self::Package(package) => ("package", package.to_v8(scope)?),
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

impl FromV8 for Object {
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
			"blob" => Self::Blob(from_v8(scope, value)?),
			"directory" => Self::Directory(from_v8(scope, value)?),
			"file" => Self::File(from_v8(scope, value)?),
			"symlink" => Self::Symlink(from_v8(scope, value)?),
			"package" => Self::Package(from_v8(scope, value)?),
			"target" => Self::Target(from_v8(scope, value)?),
			_ => unreachable!(),
		};
		Ok(value)
	}
}

impl ToV8 for blob::ArchiveFormat {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for blob::ArchiveFormat {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for blob::CompressionFormat {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for blob::CompressionFormat {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for Checksum {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for Checksum {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for checksum::Algorithm {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for checksum::Algorithm {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}

impl ToV8 for System {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for System {
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

impl ToV8 for error::Location {
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

impl FromV8 for error::Location {
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
