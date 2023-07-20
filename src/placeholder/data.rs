use super::Placeholder;

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
	pub fn from_data(placeholder: Data) -> Self {
		Self {
			name: placeholder.name,
		}
	}
}
