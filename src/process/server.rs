use crate::{
	artifact::Artifact,
	error::{Error, Result, WrapErr},
	instance::Instance,
	template::Template,
	util::{
		fs,
		http::{full, Incoming, Outgoing},
	},
};
use futures::FutureExt;
use http_body_util::BodyExt;
use itertools::Itertools;
use std::{convert::Infallible, sync::Weak};

#[derive(Clone)]
pub struct Server {
	tg: Weak<Instance>,
}

impl Server {
	#[must_use]
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
			(http::Method::POST, ["v1", "checkin"]) => {
				Some(self.handle_checkin_request(request).boxed())
			},
			(http::Method::POST, ["v1", "checkout"]) => {
				Some(self.handle_checkout_request(request).boxed())
			},
			(http::Method::POST, ["v1", "unrender"]) => {
				Some(self.handle_unrender_request(request).boxed())
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

	async fn handle_checkin_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		let tg = self.tg.upgrade().unwrap();

		// Read the request body.
		let body = request
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Failed to read the request body.")?
			.to_bytes();

		// Deserialize the path from the body.
		let path: fs::PathBuf = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the request body.")?;

		// Check in the artifact.
		let artifact = Artifact::check_in(&tg, &path)
			.await
			.wrap_err("Failed to check in the path.")?;

		// Perform an internal checkout of the artifact.
		artifact
			.check_out_internal(&tg)
			.await
			.expect("Failed to checkout artifact after checkin.");

		// Create the response.
		let body = serde_json::to_vec(&artifact.hash())
			.map_err(Error::other)
			.wrap_err("Failed to serialize the response body.")?;

		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}

	async fn handle_checkout_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		let tg = self.tg.upgrade().unwrap();

		// Read the request body.
		let body = request
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Filed to read the request body.")?
			.to_bytes();

		// Deserialize the hash.
		let hash = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the request body.")?;

		// Get the artifact.
		let artifact = Artifact::get(&tg, hash).await?;

		// Perform the internal checkout.
		let path = artifact
			.check_out_internal(&tg)
			.await
			.wrap_err("Failed to check out the artifact.")?;

		// Create the response.
		let body = serde_json::to_vec(&path)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}

	async fn handle_unrender_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		let tg = self.tg.upgrade().unwrap();

		// Read the request body.
		let body = request
			.into_body()
			.collect()
			.await
			.map_err(Error::other)
			.wrap_err("Failed to read the request body.")?
			.to_bytes();

		// Deserialize the string.
		let string: String = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the request body.")?;

		// Unrender the string.
		let artifacts_path = tg.artifacts_path();
		let template = Template::unrender(&tg, &artifacts_path, &string)
			.await
			.wrap_err("Failed to check in the path.")?;

		// Create the response.
		let body = serde_json::to_vec(&template)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the response body.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}
}
