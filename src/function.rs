use crate::package;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Function {
	#[buffalo(id = 0)]
	#[serde(rename = "packageInstanceHash")]
	pub package_instance_hash: package::instance::Hash,

	#[buffalo(id = 1)]
	pub name: String,
}
