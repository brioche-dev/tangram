#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
	pub name: Option<String>,
	pub version: Option<String>,
}
