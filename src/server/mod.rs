use crate::{
	error::{Error, Result},
	instance::Instance,
};
use ::bytes::Bytes;
use futures::{FutureExt, Stream};
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::body::{Body, Frame};
use itertools::Itertools;
use pin_project::pin_project;
use std::{
	convert::Infallible,
	future::Future,
	net::SocketAddr,
	pin::Pin,
	task::{Context, Poll},
};

// mod block;
mod error;

#[derive(Clone)]
pub struct Server {
	tg: Instance,
}

impl Server {
	#[must_use]
	pub fn new(tg: Instance) -> Self {
		Self { tg }
	}

	pub async fn serve(self, addr: SocketAddr) -> Result<()> {
		let listener = tokio::net::TcpListener::bind(&addr)
			.await
			.map_err(Error::other)?;
		tracing::info!("🚀 Serving on {}.", addr);
		loop {
			let (stream, _) = listener.accept().await?;
			let stream = TokioIo::new(stream);
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
			// (http::Method::GET, ["v1", "blocks", _]) => {
			// 	Some(self.handle_get_block_request(request).boxed())
			// },
			// (http::Method::PUT, ["v1", "blocks", _]) => {
			// 	Some(self.handle_put_block_request(request).boxed())
			// },
			// (http::Method::POST, ["v1", "operations", _, "evaluate"]) => {
			// 	Some(self.handle_operation_evaluate_request(request).boxed())
			// },
			// (http::Method::GET, ["v1", "evaluations", _]) => {
			// 	Some(self.handle_get_evaluation_request(request).boxed())
			// },
			// (http::Method::GET, ["v1", "evaluations", _, "log"]) => {
			// 	Some(self.handle_get_evaluation_request(request).boxed())
			// },
			// (http::Method::POST, ["v1", "commands", _, "execute"]) => {
			// 	Some(self.handle_command_execute_request(request).boxed())
			// },
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
