use super::full;

/// 400
pub fn bad_request() -> super::Response {
	http::Response::builder()
		.status(http::StatusCode::BAD_REQUEST)
		.body(full("Bad request."))
		.unwrap()
}

/// 404
pub fn not_found() -> super::Response {
	http::Response::builder()
		.status(http::StatusCode::NOT_FOUND)
		.body(full("Bad request."))
		.unwrap()
}
