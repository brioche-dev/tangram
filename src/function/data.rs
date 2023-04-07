use crate::package;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Data {
	#[buffalo(id = 0)]
	pub package_instance_hash: package::instance::Hash,

	#[buffalo(id = 1)]
	pub name: String,
}

impl super::Function {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			package_instance_hash: self.package_instance_hash,
			name: self.name.clone(),
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self {
			package_instance_hash: data.package_instance_hash,
			name: data.name,
		}
	}
}
