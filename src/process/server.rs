use crate::{
	error::{Error, Result},
	util::{
		fs,
		http::{full, Request, Response},
	},
	Instance,
};
use futures::FutureExt;
use std::{convert::Infallible, sync::Weak};

#[derive(Clone)]
pub struct Server {
	tg: Weak<Instance>,
}

impl Server {
	pub fn new(tg: Weak<Instance>) -> Self {
		Self { tg }
	}

	pub async fn serve(self, path: &fs::Path) -> Result<()> {
		// Bind the server's socket.
		let listener = tokio::net::UnixListener::bind(path)?;

		// Handle connections.
		loop {
			let (stream, _) = listener.accept().await?;
			let server = self.clone();
			tokio::task::spawn(async move {
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
			(http::Method::POST, ["v1", "checkin"]) => {
				Some(self.handle_checkin_request(request).boxed())
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

	async fn handle_checkin_request(&self, request: Request) -> Result<Response> {
		todo!()
	}
}
