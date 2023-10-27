pub mod addr;

pub type Incoming = hyper::body::Incoming;
pub type Outgoing = http_body_util::combinators::UnsyncBoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

/// An empty request/response body.
#[must_use]
pub fn empty() -> Outgoing {
	use http_body_util::BodyExt;
	http_body_util::Empty::new()
		.map_err(Into::into)
		.boxed_unsync()
}

/// A full request/response body.
#[must_use]
pub fn full(chunk: impl Into<::bytes::Bytes>) -> Outgoing {
	use http_body_util::BodyExt;
	http_body_util::Full::new(chunk.into())
		.map_err(Into::into)
		.boxed_unsync()
}

/// 200
#[must_use]
pub fn ok() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::OK)
		.body(empty())
		.unwrap()
}

/// 400
#[must_use]
pub fn bad_request() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::BAD_REQUEST)
		.body(full("Bad request."))
		.unwrap()
}

/// 404
#[must_use]
pub fn not_found() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::NOT_FOUND)
		.body(full("Not found."))
		.unwrap()
}
