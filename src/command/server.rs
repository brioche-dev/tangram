use crate::{
	artifact::Artifact,
	error::{return_error, Error, Result, WrapErr},
	instance::Instance,
	template::Template,
	util::http::{full, Incoming, Outgoing},
};
use futures::FutureExt;
use http_body_util::BodyExt;
use itertools::Itertools;
use std::{
	convert::Infallible,
	path::{Path, PathBuf},
	sync::Weak,
};

#[derive(Clone)]
pub struct Server {
	tg: Weak<Instance>,
	_artifacts_directory_host_path: PathBuf,
	artifacts_directory_guest_path: PathBuf,
	working_directory_host_path: PathBuf,
	working_directory_guest_path: PathBuf,
	output_host_path: PathBuf,
	output_guest_path: PathBuf,
}

impl Server {
	#[must_use]
	pub fn new(
		tg: Weak<Instance>,
		artifacts_directory_host_path: PathBuf,
		artifacts_directory_guest_path: PathBuf,
		working_directory_host_path: PathBuf,
		working_directory_guest_path: PathBuf,
		output_host_path: PathBuf,
		output_guest_path: PathBuf,
	) -> Self {
		Self {
			tg,
			_artifacts_directory_host_path: artifacts_directory_host_path,
			artifacts_directory_guest_path,
			working_directory_host_path,
			working_directory_guest_path,
			output_host_path,
			output_guest_path,
		}
	}

	pub async fn serve(self, path: &Path) -> Result<()> {
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
		let guest_path: PathBuf = serde_json::from_slice(&body)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the request body.")?;

		// Get the host path.
		let host_path =
			if let Ok(path) = guest_path.strip_prefix(&self.working_directory_guest_path) {
				if path.components().count() == 0 {
					self.working_directory_host_path.clone()
				} else {
					self.working_directory_host_path.join(path)
				}
			} else if let Ok(path) = guest_path.strip_prefix(&self.output_guest_path) {
				if path.components().count() == 0 {
					self.output_host_path.clone()
				} else {
					self.output_host_path.join(path)
				}
			} else {
				return_error!("The path is not in the artifacts, working, or output directories.");
			};

		// Check in the artifact.
		let artifact = Artifact::check_in(&tg, &host_path)
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
		let template = Template::unrender(&tg, &self.artifacts_directory_guest_path, &string)
			.await
			.wrap_err("Failed to unrender the template.")?
			.to_data();

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
