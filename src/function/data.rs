use crate::{package, path::Subpath};

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Data {
	#[buffalo(id = 0)]
	pub package_instance_hash: package::instance::Hash,

	#[buffalo(id = 1)]
	pub module_path: Subpath,

	#[buffalo(id = 2)]
	pub name: String,
}

impl super::Function {
	#[must_use]
	pub fn to_data(&self) -> Data {
		Data {
			package_instance_hash: self.package_instance_hash,
			module_path: self.module_path.clone(),
			name: self.name.clone(),
		}
	}

	#[must_use]
	pub fn from_data(data: Data) -> Self {
		Self {
			package_instance_hash: data.package_instance_hash,
			module_path: data.module_path,
			name: data.name,
		}
	}
}
