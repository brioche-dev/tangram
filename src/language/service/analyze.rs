use crate::{module, path::Relpath};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub text: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub imports: Vec<module::Import>,
	pub includes: Vec<Relpath>,
}
