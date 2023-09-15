crate::id!(Placeholder);

/// A placeholder handle.
#[derive(Clone, Debug)]
pub struct Handle(crate::Handle);

crate::handle!(Placeholder);

/// A placeholder value.
#[derive(Clone, Debug)]
pub struct Value {
	pub name: String,
}

crate::value!(Placeholder);

/// Placeholder data.
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
	pub name: String,
}

impl Value {
	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self { name: data.name }
	}

	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			name: self.name.clone(),
		}
	}
}

// impl std::fmt::Display for Value {
// 	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// 		write!(f, r#"(tg.placeholder {})"#, self.name)?;
// 		Ok(())
// 	}
// }
