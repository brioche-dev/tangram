use crate::{
	error::{Error, Result},
	util::http::{full, Request, Response},
	Instance,
};
use futures::FutureExt;
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

mod artifact;
mod blob;
mod error;

impl Instance {
	pub async fn serve(self: &Arc<Self>, addr: SocketAddr) -> Result<()> {
		let tg = Arc::clone(self);
		let listener = tokio::net::TcpListener::bind(&addr)
			.await
			.map_err(Error::other)?;
		tracing::info!("🚀 Serving on {}.", addr);
		while let (stream, _) = listener.accept().await? {
			let tg = Arc::clone(&tg);
			tokio::spawn(async move {
				hyper::server::conn::http1::Builder::new()
					.serve_connection(
						stream,
						hyper::service::service_fn(move |request| {
							let tg = Arc::clone(&tg);
							async move {
								let response = tg.handle_request(request).await;
								Ok::<_, Infallible>(response)
							}
						}),
					)
					.await
					.ok()
			});
		}
		Ok(())
	}

	async fn handle_request(&self, request: Request) -> Response {
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

	async fn handle_request_inner(&self, request: Request) -> Result<Option<Response>> {
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
			Some(response.await.map_err(Error::other)?)
		} else {
			None
		};
		Ok(response)
	}
}
