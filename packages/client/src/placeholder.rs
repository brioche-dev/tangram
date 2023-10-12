/// A placeholder.
#[derive(Clone, Debug)]
pub struct Placeholder {
	pub name: String,
}

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

impl Placeholder {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			name: self.name.clone(),
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self { name: data.name }
	}
}
