use crate::{return_error, Error, Result, Server};
use futures::{FutureExt, TryStreamExt};
use http_body_util::BodyExt;
use itertools::Itertools;
use std::{convert::Infallible, net::SocketAddr};
use tokio::io::AsyncReadExt;

impl Server {
	pub async fn serve(self, addr: SocketAddr) -> Result<()> {
		let listener = tokio::net::TcpListener::bind(&addr)
			.await
			.map_err(Error::other)?;
		tracing::info!("🚀 Serving on {}.", addr);
		loop {
			let (stream, _) = listener.accept().await?;
			let stream = hyper_util::rt::TokioIo::new(stream);
			let server = self.clone();
			tokio::spawn(async move {
				hyper::server::conn::http2::Builder::new(hyper_util::rt::TokioExecutor::new())
					.serve_connection(
						stream,
						hyper::service::service_fn(move |request| {
							let server = server.clone();
							async move {
								let response = server.handle_request(request).await;
								Ok::<_, Infallible>(response)
							}
						}),
					)
					.await
					.ok()
			});
		}
	}

	async fn handle_request(&self, request: http::Request<Incoming>) -> http::Response<Outgoing> {
		match self.handle_request_inner(request).await {
			Ok(Some(response)) => response,
			Ok(None) => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(full("Not found."))
				.unwrap(),
			Err(error) => {
				tracing::error!(?error);
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(full("Internal server error."))
					.unwrap()
			},
		}
	}

	async fn handle_request_inner(
		&self,
		request: http::Request<Incoming>,
	) -> Result<Option<http::Response<Outgoing>>> {
		let method = request.method().clone();
		let path = request.uri().path().to_owned();
		let path_components = path.split('/').skip(1).collect_vec();
		let response = match (method, path_components.as_slice()) {
			(http::Method::HEAD, ["v1", "objects", _]) => {
				Some(self.handle_head_object_request(request).boxed())
			},
			(http::Method::GET, ["v1", "objects", _]) => {
				Some(self.handle_get_object_request(request).boxed())
			},
			(http::Method::PUT, ["v1", "objects", _]) => {
				Some(self.handle_put_object_request(request).boxed())
			},
			(_, _) => None,
		};
		let response = if let Some(response) = response {
			Some(response.await.map_err(Error::other)?)
		} else {
			None
		};
		Ok(response)
	}

	pub async fn handle_head_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		let status = if self.get_object_exists(id).await? {
			http::StatusCode::OK
		} else {
			http::StatusCode::NOT_FOUND
		};

		// Create the response.
		let response = http::Response::builder()
			.status(status)
			.body(empty())
			.unwrap();

		Ok(response)
	}

	pub async fn handle_get_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		let bytes = self.try_get_object_bytes(id).await?;

		let Some(bytes) = bytes else {
			return Ok(http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(empty())
				.unwrap());
		};

		// Create the body.
		let body = full(bytes);

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body)
			.unwrap();

		Ok(response)
	}

	pub async fn handle_put_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Create a reader from the body.
		let mut body = tokio_util::io::StreamReader::new(
			http_body_util::BodyStream::new(request.into_body())
				.try_filter_map(|frame| Box::pin(async move { Ok(frame.into_data().ok()) }))
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
		);

		// Read the body.
		let mut bytes = Vec::new();
		body.read_to_end(&mut bytes).await?;

		// Put the object.
		let result = self.try_put_object_bytes(id, &bytes).await?;

		// If there are missing children, then return a bad request response.
		if let Err(missing_children) = result {
			let body = serde_json::to_vec(&missing_children).map_err(Error::other)?;
			let response = http::Response::builder()
				.status(http::StatusCode::BAD_REQUEST)
				.body(full(body))
				.unwrap();
			return Ok(response);
		}

		// Otherwise, return an ok response.
		Ok(http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap())
	}
}

pub type Incoming = hyper::body::Incoming;
pub type Outgoing = http_body_util::combinators::BoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

/// An empty response body.
#[must_use]
pub fn empty() -> Outgoing {
	http_body_util::Empty::new()
		.map_err(|_| unreachable!())
		.boxed()
}

/// A full response body.
#[must_use]
pub fn full(chunk: impl Into<::bytes::Bytes>) -> Outgoing {
	http_body_util::Full::new(chunk.into())
		.map_err(|_| unreachable!())
		.boxed()
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
