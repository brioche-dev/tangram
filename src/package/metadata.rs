#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Metadata {
	pub name: Option<String>,
	pub version: Option<String>,
}
