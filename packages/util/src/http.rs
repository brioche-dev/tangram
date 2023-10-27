use std::collections::BTreeMap;
use tangram_error::{Result, WrapErr};

pub type Incoming = hyper::body::Incoming;

pub type Outgoing = http_body_util::combinators::UnsyncBoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

/// Get a bearer token or cookie from an HTTP request.
pub fn get_token(request: &http::Request<Incoming>, name: Option<&str>) -> Option<String> {
	if let Some(authorization) = request.headers().get(http::header::AUTHORIZATION) {
		let authorization = match authorization.to_str() {
			Ok(authorization) => authorization,
			Err(_) => return None,
		};
		let mut components = authorization.split(' ');
		let token = match (components.next(), components.next()) {
			(Some("Bearer"), Some(token)) => token.to_owned(),
			_ => return None,
		};
		Some(token)
	} else if let Some(cookies) = request.headers().get(http::header::COOKIE) {
		if let Some(name) = name {
			let cookies = match cookies.to_str() {
				Ok(cookies) => cookies,
				Err(_) => return None,
			};
			let cookies: BTreeMap<&str, &str> = match parse_cookies(cookies).collect() {
				Ok(cookies) => cookies,
				Err(_) => return None,
			};
			let token = match cookies.get(name) {
				Some(&token) => token.to_owned(),
				None => return None,
			};
			Some(token)
		} else {
			None
		}
	} else {
		None
	}
}

/// Parse an HTTP cookie string.
pub fn parse_cookies(cookies: &str) -> impl Iterator<Item = Result<(&str, &str)>> {
	cookies.split("; ").map(|cookie| {
		let mut components = cookie.split('=');
		let key = components
			.next()
			.wrap_err("Expected a key in the cookie string.")?;
		let value = components
			.next()
			.wrap_err("Expected a value in the cookie string.")?;
		Ok((key, value))
	})
}

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

/// 401
#[must_use]
pub fn unauthorized() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::UNAUTHORIZED)
		.body(full("Unauthorized."))
		.unwrap()
}

/// 403
#[must_use]
pub fn forbidden() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::FORBIDDEN)
		.body(full("Forbidden."))
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
