use async_trait::async_trait;
use bytes::Bytes;
use futures::{Stream, StreamExt, TryStreamExt};
use http::Response;
use http_body::Frame;
use tokio::io::AsyncReadExt;

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
		.map_err(|_| unreachable!())
		.boxed_unsync()
}

/// A full request/response body.
#[must_use]
pub fn full(chunk: impl Into<::bytes::Bytes>) -> Outgoing {
	use http_body_util::BodyExt;
	http_body_util::Full::new(chunk.into())
		.map_err(|_| unreachable!())
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

pub fn bytes_stream(body: Incoming) -> impl Stream<Item = hyper::Result<Bytes>> {
	use hyper::body::Body;
	let mut body = Box::pin(body);
	let stream = futures::stream::poll_fn(move |cx| {
		let body = body.as_mut();
		body.poll_frame(cx)
	})
	.filter_map(|frame| async {
		match frame.map(Frame::into_data) {
			Ok(Ok(bytes)) => Some(Ok(bytes)),
			Err(e) => Some(Err(e)),
			Ok(Err(_frame)) => None,
		}
	});
	stream
}

/// 404
#[must_use]
pub fn not_found() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::NOT_FOUND)
		.body(full("Not found."))
		.unwrap()
}

#[async_trait]
pub trait BodyExt {
	async fn json<T>(self) -> Result<T, serde_json::Error>
	where
		T: serde::de::DeserializeOwned;
	async fn bytes(self) -> Result<Bytes, http::Error>;
}

pub trait ResponseExt {
	fn error_for_status(self) -> Result<Self, http::StatusCode>
	where
		Self: Sized;
}

impl ResponseExt for Response<Incoming> {
	fn error_for_status(self) -> Result<Self, http::StatusCode> {
		if self.status().is_success() {
			Ok(self)
		} else {
			Err(self.status())
		}
	}
}

#[async_trait]
impl BodyExt for http::Response<Incoming> {
	async fn json<T>(self) -> Result<T, serde_json::Error>
	where
		T: serde::de::DeserializeOwned,
	{
		let mut body = tokio_util::io::StreamReader::new(
			http_body_util::BodyStream::new(self.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);
		let mut bytes = Vec::new();
		body.read_to_end(&mut bytes).await.unwrap(); // TODO: move errors to this crate.
		serde_json::from_slice(&bytes)
	}

	async fn bytes(self) -> Result<Bytes, http::Error> {
		let mut body = tokio_util::io::StreamReader::new(
			http_body_util::BodyStream::new(self.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);
		let mut bytes = Vec::new();
		body.read_to_end(&mut bytes).await.unwrap(); // TODO: move errors to this crate.
		Ok(bytes.into())
	}
}

#[async_trait]
impl BodyExt for http::Request<Incoming> {
	async fn json<T>(self) -> Result<T, serde_json::Error>
	where
		T: serde::de::DeserializeOwned,
	{
		let mut body = tokio_util::io::StreamReader::new(
			http_body_util::BodyStream::new(self.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);
		let mut bytes = Vec::new();
		body.read_to_end(&mut bytes).await.unwrap(); // TODO move errors to this crate.
		serde_json::from_slice(&bytes)
	}

	async fn bytes(self) -> Result<Bytes, http::Error> {
		let mut body = tokio_util::io::StreamReader::new(
			http_body_util::BodyStream::new(self.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);
		let mut bytes = Vec::new();
		body.read_to_end(&mut bytes).await.unwrap(); // TODO: move errors to this crate.
		Ok(bytes.into())
	}
}
