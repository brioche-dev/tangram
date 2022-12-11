/// 400
pub fn bad_request() -> http::Response<hyper::Body> {
	http::Response::builder()
		.status(http::StatusCode::BAD_REQUEST)
		.body(hyper::Body::from("Bad request."))
		.unwrap()
}

/// 404
pub fn not_found() -> http::Response<hyper::Body> {
	http::Response::builder()
		.status(http::StatusCode::NOT_FOUND)
		.body(hyper::Body::from("Not found."))
		.unwrap()
}
