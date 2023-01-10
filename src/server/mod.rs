use crate::Cli;
use anyhow::Result;
use futures::FutureExt;
use std::{convert::Infallible, net::SocketAddr};

pub mod artifact;
mod blob;
mod error;

#[derive(Clone)]
pub struct Server {
	cli: Cli,
}

impl Server {
	#[must_use]
	pub fn new(cli: Cli) -> Server {
		Server { cli }
	}

	pub async fn serve(self, addr: SocketAddr) -> Result<()> {
		let server = self;
		hyper::Server::try_bind(&addr)
			.map(|server| {
				tracing::info!("🚀 Serving on {}.", addr);
				server
			})?
			.serve(hyper::service::make_service_fn(move |_| {
				let server = server.clone();
				async move {
					Ok::<_, Infallible>(hyper::service::service_fn(move |request| {
						let server = server.clone();
						async move {
							let response = server.handle_request_wrapper(request).await;
							Ok::<_, Infallible>(response)
						}
					}))
				}
			}))
			.await?;
		Ok(())
	}

	pub async fn handle_request_wrapper(
		&self,
		request: http::Request<hyper::Body>,
	) -> http::Response<hyper::Body> {
		match self.handle_request(request).await {
			Ok(Some(response)) => response,
			Ok(None) => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(hyper::Body::from("Not found."))
				.unwrap(),
			Err(error) => {
				tracing::error!(?error, backtrace = %error.backtrace());
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(hyper::Body::from(format!("{error:?}")))
					.unwrap()
			},
		}
	}

	pub async fn handle_request(
		&self,
		request: http::Request<hyper::Body>,
	) -> Result<Option<http::Response<hyper::Body>>> {
		let method = request.method().clone();
		let path = request.uri().path().to_owned();
		let path_components = path.split('/').skip(1).collect::<Vec<_>>();
		let response = match (method, path_components.as_slice()) {
			(http::Method::GET, ["v1", "blobs", _]) => {
				Some(self.handle_get_blob_request(request).boxed())
			},
			(http::Method::POST, ["v1", "blobs", ""]) => {
				Some(self.handle_add_blob_request(request).boxed())
			},
			(http::Method::GET, ["v1", "artifacts", _]) => {
				Some(self.handle_get_artifact_request(request).boxed())
			},
			(http::Method::POST, ["v1", "artifacts", ""]) => {
				Some(self.handle_add_artifact_request(request).boxed())
			},
			(_, _) => None,
		};
		let response = if let Some(response) = response {
			Some(response.await?)
		} else {
			None
		};
		Ok(response)
	}
}
