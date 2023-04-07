use crate::{
	error::{Error, Result},
	instance::Instance,
	util::http::{full, Incoming, Outgoing},
};
use futures::FutureExt;
use itertools::Itertools;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

mod artifact;
mod blob;
mod error;

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
			(http::Method::GET, ["v1", "blobs", _]) => {
				Some(self.handle_get_blob_request(request).boxed())
			},
			(http::Method::POST, ["v1", "blobs", ""]) => {
				Some(self.handle_add_blob_request(request).boxed())
			},
			(http::Method::GET, ["v1", "artifacts", _]) => {
				Some(self.handle_get_artifact_request(request).boxed())
			},
			(http::Method::POST, ["v1", "artifacts", _]) => {
				Some(self.handle_add_artifact_request(request).boxed())
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
