use super::full;
use crate::util::http::Outgoing;

/// 400
pub fn bad_request() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::BAD_REQUEST)
		.body(full("Bad request."))
		.unwrap()
}

/// 404
pub fn not_found() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::NOT_FOUND)
		.body(full("Bad request."))
		.unwrap()
}
