use crate::{
	error::{Error, Result},
	instance::Instance,
};
use bytes::Bytes;
use futures::FutureExt;
use futures::Stream;
pub use http_body_util::StreamBody;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::{Body, Frame};
use itertools::Itertools;
use pin_project::pin_project;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use std::{
	pin::Pin,
	task::{Context, Poll},
};

mod artifact;
mod blob;
mod error;
mod operation;

#[derive(Clone)]
pub struct Server {
	tg: Arc<Instance>,
}

impl Server {
	pub fn new(tg: Arc<Instance>) -> Self {
		Self { tg }
	}

	pub async fn serve(self, addr: SocketAddr) -> Result<()> {
		let listener = tokio::net::TcpListener::bind(&addr)
			.await
			.map_err(Error::other)?;
		tracing::info!("🚀 Serving on {}.", addr);
		loop {
			let (stream, _) = listener.accept().await?;
			let server = self.clone();
			tokio::spawn(async move {
				hyper::server::conn::http1::Builder::new()
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
			(http::Method::GET, ["v1", "artifacts", _]) => {
				Some(self.handle_get_artifact_request(request).boxed())
			},
			(http::Method::POST, ["v1", "artifacts", _]) => {
				Some(self.handle_post_artifact_request(request).boxed())
			},
			(http::Method::GET, ["v1", "blobs", _]) => {
				Some(self.handle_get_blob_request(request).boxed())
			},
			(http::Method::POST, ["v1", "blobs"]) => {
				Some(self.handle_post_blob_request(request).boxed())
			},
			(http::Method::GET, ["v1", "operations", _]) => {
				Some(self.handle_get_operation_request(request).boxed())
			},
			(http::Method::POST, ["v1", "operations"]) => {
				Some(self.handle_post_operation_request(request).boxed())
			},
			(http::Method::GET, ["v1", "operations", _, "output"]) => {
				Some(self.handle_get_operation_output_request(request).boxed())
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
}

pub type Incoming = hyper::body::Incoming;
pub type Outgoing = BoxBody<Bytes, Box<dyn std::error::Error + Send + Sync + 'static>>;

#[must_use]
pub fn empty() -> Outgoing {
	Empty::new().map_err(|_| unreachable!()).boxed()
}

pub fn full(chunk: impl Into<Bytes>) -> Outgoing {
	Full::new(chunk.into()).map_err(|_| unreachable!()).boxed()
}

#[pin_project]
pub struct BodyStream<B> {
	#[pin]
	body: B,
}

impl<B> BodyStream<B> {
	pub fn new(body: B) -> BodyStream<B> {
		BodyStream { body }
	}
}

impl<B> Body for BodyStream<B>
where
	B: Body,
{
	type Data = B::Data;
	type Error = B::Error;

	fn poll_frame(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
		self.project().body.poll_frame(cx)
	}
}
impl<B> Stream for BodyStream<B>
where
	B: Body,
{
	type Item = Result<Frame<B::Data>, B::Error>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		match self.project().body.poll_frame(cx) {
			Poll::Ready(Some(frame)) => Poll::Ready(Some(frame)),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}
