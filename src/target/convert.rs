use crate::error::{return_error, Error, Result, WrapErr};
use num_traits::ToPrimitive;
use std::collections::BTreeMap;

pub fn to_v8<'a, T>(scope: &mut v8::HandleScope<'a>, value: &T) -> Result<v8::Local<'a, v8::Value>>
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
		let value = v8::Local::<v8::Boolean>::try_from(value)
			.map_err(Error::other)
			.wrap_err("Expected a boolean value.")?;
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
			.map_err(Error::other)?
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
		Ok(value
			.to_string(scope)
			.wrap_err("Expected a string.")?
			.to_rust_string_lossy(scope))
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
		let value = v8::Local::<v8::Array>::try_from(value)
			.map_err(Error::other)
			.wrap_err("Expected an array.")?;
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
		let value = v8::Local::<v8::Array>::try_from(value)
			.map_err(Error::other)
			.wrap_err("Expected an array.")?;
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
		let value = v8::Local::<v8::Array>::try_from(value)
			.map_err(Error::other)
			.wrap_err("Expected an array.")?;
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
		let value = v8::Local::<v8::Object>::try_from(value)
			.map_err(Error::other)
			.wrap_err("Expected an object.")?;
		let property_names = value
			.get_own_property_names(scope, v8::GetPropertyNamesArgs::default())
			.unwrap();
		let mut output = BTreeMap::default();
		for i in 0..property_names.length() {
			let key = property_names.get_index(scope, i).unwrap();
			let value = value.get(scope, key).unwrap();
			let key = key.to_rust_string_lossy(scope);
			let value = from_v8(scope, value)?;
			output.insert(key, value);
		}
		Ok(output)
	}
}

impl ToV8 for serde_json::Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		serde_v8::to_v8(scope, self).map_err(Error::other)
	}
}

impl FromV8 for serde_json::Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		serde_v8::from_v8(scope, value).map_err(Error::other)
	}
}

impl ToV8 for serde_toml::Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		serde_v8::to_v8(scope, self).map_err(Error::other)
	}
}

impl FromV8 for serde_toml::Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		serde_v8::from_v8(scope, value).map_err(Error::other)
	}
}

impl ToV8 for serde_yaml::Value {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		serde_v8::to_v8(scope, self).map_err(Error::other)
	}
}

impl FromV8 for serde_yaml::Value {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		serde_v8::from_v8(scope, value).map_err(Error::other)
	}
}
