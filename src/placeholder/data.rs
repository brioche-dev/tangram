use super::Placeholder;

#[derive(
	Clone,
	Debug,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
	serde::Deserialize,
	serde::Serialize,
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
