#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
	pub text: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
	pub transpiled_text: String,
	pub source_map_string: String,
}
