use crate::Server;
use bytes::Bytes;
use futures::{
	future::{self},
	FutureExt, TryStreamExt,
};
use http_body_util::{BodyExt, StreamBody};
use hyper_util::rt::{TokioExecutor, TokioIo};
use itertools::Itertools;
use std::{collections::BTreeMap, convert::Infallible};
use tangram_client as tg;
use tangram_error::{return_error, Result, WrapErr};
use tg::Handle;
use tokio::net::{TcpListener, UnixListener};
use tokio_util::either::Either;

type Incoming = hyper::body::Incoming;

type Outgoing = http_body_util::combinators::UnsyncBoxBody<
	::bytes::Bytes,
	Box<dyn std::error::Error + Send + Sync + 'static>,
>;

impl Server {
	pub async fn serve(self, addr: tg::client::Addr) -> Result<()> {
		// Create the listener.
		let listener = match &addr {
			tg::client::Addr::Inet(inet) => Either::Left(
				TcpListener::bind(inet.to_string())
					.await
					.wrap_err("Failed to create the TCP listener.")?,
			),
			tg::client::Addr::Unix(path) => Either::Right(
				UnixListener::bind(path).wrap_err("Failed to create the UNIX listener.")?,
			),
		};

		tracing::info!("🚀 Serving on {addr:?}.");

		// Loop forever, accepting connections.
		loop {
			// Accept a new connection.
			let stream = TokioIo::new(match &listener {
				Either::Left(listener) => Either::Left(
					listener
						.accept()
						.await
						.wrap_err("Failed to accept a new TCP connection.")?
						.0,
				),
				Either::Right(listener) => Either::Right(
					listener
						.accept()
						.await
						.wrap_err("Failed to accept a new UNIX connection.")?
						.0,
				),
			});

			// Create the service.
			let service = hyper::service::service_fn({
				let server = self.clone();
				move |request| {
					let server = server.clone();
					async move { Ok::<_, Infallible>(server.handle_request(request).await) }
				}
			});

			// Spawn the connection.
			tokio::spawn(async move {
				let builder = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new());
				let connection = builder.serve_connection(stream, service);
				if let Err(error) = connection.await {
					tracing::error!(?error, "Failed to serve the connection.");
				}
			});
		}
	}

	async fn try_get_user_from_request(
		&self,
		request: &http::Request<Incoming>,
	) -> Result<Option<tg::user::User>> {
		// Get the token.
		let Some(token) = get_token(request, None) else {
			return Ok(None);
		};

		// Get the user.
		let user = self.get_user_for_token(&token).await?;

		Ok(user)
	}

	#[allow(clippy::too_many_lines)]
	async fn handle_request(&self, request: http::Request<Incoming>) -> http::Response<Outgoing> {
		tracing::info!(method = ?request.method(), path = ?request.uri().path(), "Received request.");

		let method = request.method().clone();
		let path_components = request.uri().path().split('/').skip(1).collect_vec();
		let response = match (method, path_components.as_slice()) {
			// Server
			(http::Method::GET, ["v1", "status"]) => {
				self.handle_get_status_request(request).map(Some).boxed()
			},
			(http::Method::POST, ["v1", "stop"]) => {
				self.handle_post_stop_request(request).map(Some).boxed()
			},
			(http::Method::POST, ["v1", "clean"]) => {
				self.handle_post_clean_request(request).map(Some).boxed()
			},

			// Builds
			(http::Method::GET, ["v1", "targets", _, "build"]) => self
				.handle_get_build_for_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "targets", _, "build"]) => self
				.handle_get_or_create_build_for_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", "queue"]) => self
				.handle_get_build_queue_item_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "target"]) => self
				.handle_get_build_target_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "children"]) => self
				.handle_get_build_children_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "builds", _, "children"]) => self
				.handle_post_build_child_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "log"]) => {
				self.handle_get_build_log_request(request).map(Some).boxed()
			},
			(http::Method::POST, ["v1", "builds", _, "log"]) => self
				.handle_post_build_log_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "builds", _, "outcome"]) => self
				.handle_get_build_outcome_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "builds", _, "cancel"]) => self
				.handle_post_build_cancel_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "builds", _, "finish"]) => self
				.handle_post_build_finish_request(request)
				.map(Some)
				.boxed(),

			// Objects
			(http::Method::HEAD, ["v1", "objects", _]) => {
				self.handle_head_object_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "objects", _]) => {
				self.handle_get_object_request(request).map(Some).boxed()
			},
			(http::Method::PUT, ["v1", "objects", _]) => {
				self.handle_put_object_request(request).map(Some).boxed()
			},

			// Packages
			(http::Method::GET, ["v1", "packages", "search"]) => self
				.handle_search_packages_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "packages", _]) => {
				self.handle_get_package_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "packages", _, "versions", _]) => self
				.handle_get_package_version_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "packages", _, "metadata"]) => self
				.handle_get_package_metadata_request(request)
				.map(Some)
				.boxed(),
			(http::Method::GET, ["v1", "packages", _, "dependencies"]) => self
				.handle_get_package_dependencies_request(request)
				.map(Some)
				.boxed(),
			(http::Method::POST, ["v1", "packages"]) => self
				.handle_publish_package_request(request)
				.map(Some)
				.boxed(),

			// Users
			(http::Method::POST, ["v1", "logins"]) => {
				self.handle_create_login_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "logins", _]) => {
				self.handle_get_login_request(request).map(Some).boxed()
			},
			(http::Method::GET, ["v1", "user"]) => self
				.handle_get_user_for_token_request(request)
				.map(Some)
				.boxed(),

			(_, _) => future::ready(None).boxed(),
		}
		.await;

		let response = match response {
			None => http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(full("Not found."))
				.unwrap(),
			Some(Err(error)) => {
				let trace = error.trace();
				tracing::error!(%trace);
				http::Response::builder()
					.status(http::StatusCode::INTERNAL_SERVER_ERROR)
					.body(full("Internal server error."))
					.unwrap()
			},
			Some(Ok(response)) => response,
		};

		tracing::info!(status = ?response.status(), "Sending response.");

		response
	}

	async fn handle_get_status_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		let status = self.status().await?;
		let body = serde_json::to_vec(&status).unwrap();
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	async fn handle_post_stop_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		self.stop().await?;
		Ok(ok())
	}

	async fn handle_post_clean_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		self.clean().await?;
		Ok(http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap())
	}

	async fn handle_get_build_queue_item_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		#[derive(serde::Deserialize)]
		struct SearchParams {
			#[serde(default)]
			systems: Option<Vec<tg::System>>,
		}
		// Get the user.
		let user = self.try_get_user_from_request(&request).await?;

		// Get the search params.
		let systems = if let Some(query) = request.uri().query() {
			let search_params: SearchParams =
				serde_urlencoded::from_str(query).wrap_err("Failed to parse the search params.")?;
			search_params.systems
		} else {
			None
		};

		let build_id = self.get_build_from_queue(user.as_ref(), systems).await?;

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the ID.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	async fn handle_get_build_for_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "targets", id, "build"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the build for the target.
		let Some(build_id) = self.try_get_build_for_target(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	async fn handle_get_or_create_build_for_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "targets", id, "build"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the search params.
		#[derive(serde::Deserialize)]
		struct SearchParams {
			#[serde(default)]
			depth: u64,
			#[serde(default)]
			retry: tg::build::Retry,
		}
		let Some(query) = request.uri().query() else {
			return Ok(bad_request());
		};
		let search_params: SearchParams =
			serde_urlencoded::from_str(query).wrap_err("Failed to parse the search params.")?;
		let depth = search_params.depth;
		let retry = search_params.retry;

		// Get the user.
		let user = self.try_get_user_from_request(&request).await?;

		// Get or create the build for the target.
		let build_id = self
			.get_or_create_build_for_target(user.as_ref(), &id, depth, retry)
			.await?;

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	async fn handle_post_build_cancel_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", build_id, "cancel"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id = build_id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the user.
		let user = self.try_get_user_from_request(&request).await?;

		self.cancel_build(user.as_ref(), &build_id).await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();

		Ok(response)
	}

	async fn handle_get_build_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "target"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the build target.
		let Some(build_id) = self.try_get_build_target(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	async fn handle_get_build_children_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "children"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the children.
		let Some(children) = self.try_get_build_children(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = Outgoing::new(StreamBody::new(
			children
				.map_ok(|id| {
					let mut id = serde_json::to_string(&id).unwrap();
					id.push('\n');
					hyper::body::Frame::data(Bytes::from(id))
				})
				.map_err(Into::into),
		));
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body)
			.unwrap();
		Ok(response)
	}

	async fn handle_post_build_child_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "children"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id: tg::build::Id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the user.
		let user = self.try_get_user_from_request(&request).await?;

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let child_id =
			serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;

		self.add_build_child(user.as_ref(), &build_id, &child_id)
			.await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();
		Ok(response)
	}

	async fn handle_get_build_log_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "log"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the log.
		let Some(log) = self.try_get_build_log(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let body = Outgoing::new(StreamBody::new(
			log.map_ok(hyper::body::Frame::data).map_err(Into::into),
		));
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(body)
			.unwrap();
		Ok(response)
	}

	async fn handle_post_build_log_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "log"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the user.
		let user = self.try_get_user_from_request(&request).await?;

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();

		self.add_build_log(user.as_ref(), &build_id, bytes).await?;

		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();
		Ok(response)
	}

	async fn handle_get_build_outcome_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "outcome"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the outcome.
		let Some(outcome) = self.try_get_build_outcome(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let outcome = outcome.data(self).await?;
		let body = serde_json::to_vec(&outcome).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	async fn handle_post_build_finish_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", build_id, "finish"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let build_id = build_id.parse().wrap_err("Failed to parse the ID.")?;

		// Get the user.
		let user = self.try_get_user_from_request(&request).await?;

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let result = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize.")?;

		// Finish the build.
		self.finish_build(user.as_ref(), &build_id, result).await?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(empty())
			.unwrap();
		Ok(response)
	}

	async fn handle_head_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Get whether the object exists.
		let exists = self.get_object_exists(&id).await?;

		// Create the response.
		let status = if exists {
			http::StatusCode::OK
		} else {
			http::StatusCode::NOT_FOUND
		};
		let response = http::Response::builder()
			.status(status)
			.body(empty())
			.unwrap();

		Ok(response)
	}

	async fn handle_get_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Get the object.
		let Some(bytes) = self.try_get_object(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(bytes))
			.unwrap();

		Ok(response)
	}

	async fn handle_put_object_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "objects", id] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();

		// Put the object.
		let result = self.try_put_object(&id, &bytes).await?;

		// If there are missing children, then return a bad request response.
		if let Err(missing_children) = result {
			let body = serde_json::to_vec(&missing_children)
				.wrap_err("Failed to serialize the missing children.")?;
			let response = http::Response::builder()
				.status(http::StatusCode::BAD_REQUEST)
				.body(full(body))
				.unwrap();
			return Ok(response);
		}

		// Otherwise, return an ok response.
		Ok(ok())
	}

	async fn handle_search_packages_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the search params.
		#[derive(serde::Deserialize, Default)]
		struct SearchParams {
			query: String,
		}
		let Some(query) = request.uri().query() else {
			return Ok(bad_request());
		};
		let search_params: SearchParams =
			serde_urlencoded::from_str(query).wrap_err("Failed to parse the search params.")?;

		// Perform the search.
		let packages = self.search_packages(&search_params.query).await?;

		// Create the response.
		let body = serde_json::to_vec(&packages).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();

		Ok(response)
	}

	async fn handle_get_package_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, _, "packages", dependency] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let dependency = dependency
			.parse()
			.wrap_err("Failed to parse the dependency.")?;

		// Get the package.
		let Some(id) = self.try_get_package(&dependency).await? else {
			return Ok(not_found());
		};

		// Create the body.
		let body = serde_json::to_vec(&id).wrap_err("Failed to serialize the ID.")?;

		// Create the response.
		let response = http::Response::builder().body(full(body)).unwrap();

		Ok(response)
	}

	async fn handle_get_package_version_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, _, "packages", dependency, "versions"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let dependency = dependency
			.parse()
			.wrap_err("Failed to parse the dependency.")?;

		// Get the package.
		let source_artifact_hash = self.try_get_package_versions(&dependency).await?;

		// Create the response.
		let response = if let Some(source_artifact_hash) = source_artifact_hash {
			let body = serde_json::to_vec(&source_artifact_hash)
				.wrap_err("Failed to serialize the source artifact hash.")?;
			http::Response::builder().body(full(body)).unwrap()
		} else {
			not_found()
		};

		Ok(response)
	}

	async fn handle_get_package_metadata_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "packages", dependency, "metadata"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let dependency = dependency
			.parse()
			.wrap_err("Failed to parse the dependency.")?;

		// Get the package metadata.
		let metadata = self.try_get_package_metadata(&dependency).await?;

		match metadata {
			Some(metadata) => {
				// Create the body.
				let body =
					serde_json::to_vec(&metadata).wrap_err("Failed to serialize the metadata.")?;

				// Create the response.
				let response = http::Response::builder().body(full(body)).unwrap();

				Ok(response)
			},
			None => Ok(http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(empty())
				.unwrap()),
		}
	}

	async fn handle_get_package_dependencies_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "packages", dependency, "dependencies"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let dependency = dependency
			.parse()
			.wrap_err("Failed to parse the dependency.")?;

		// Get the package dependencies.
		let dependencies = self.try_get_package_dependencies(&dependency).await?;

		match dependencies {
			Some(dependencies) => {
				// Create the body.
				let body = serde_json::to_vec(&dependencies)
					.wrap_err("Failed to serialize the package.")?;

				// Create the response.
				let response = http::Response::builder().body(full(body)).unwrap();

				Ok(response)
			},
			None => Ok(http::Response::builder()
				.status(http::StatusCode::NOT_FOUND)
				.body(empty())
				.unwrap()),
		}
	}

	async fn handle_publish_package_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the user.
		let user = self.try_get_user_from_request(&request).await?;

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let package_id = serde_json::from_slice(&bytes).wrap_err("Invalid request.")?;

		// Create the package.
		self.publish_package(user.as_ref(), &package_id).await?;

		Ok(ok())
	}

	async fn handle_create_login_request(
		&self,
		_request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Create the login.
		let login = self.create_login().await?;

		// Create the response.
		let body = serde_json::to_string(&login).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(200)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	async fn handle_get_login_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "logins", id] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let Ok(id) = id.parse() else {
			return Ok(bad_request());
		};

		// Get the login.
		let login = self.get_login(&id).await?;

		// Create the response.
		let response =
			serde_json::to_string(&login).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(200)
			.body(full(response))
			.unwrap();
		Ok(response)
	}

	async fn handle_get_user_for_token_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Get the token from the request.
		let Some(token) = get_token(&request, None) else {
			return Ok(unauthorized());
		};

		// Authenticate the user.
		let Some(user) = self.get_user_for_token(token.as_str()).await? else {
			return Ok(unauthorized());
		};

		// Create the response.
		let body = serde_json::to_string(&user).wrap_err("Failed to serialize the user.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}
}

#[must_use]
pub fn empty() -> Outgoing {
	http_body_util::Empty::new()
		.map_err(Into::into)
		.boxed_unsync()
}

#[must_use]
pub fn full(chunk: impl Into<::bytes::Bytes>) -> Outgoing {
	http_body_util::Full::new(chunk.into())
		.map_err(Into::into)
		.boxed_unsync()
}

/// Get a bearer token or cookie from an HTTP request.
pub fn get_token(request: &http::Request<Incoming>, name: Option<&str>) -> Option<String> {
	if let Some(authorization) = request.headers().get(http::header::AUTHORIZATION) {
		let Ok(authorization) = authorization.to_str() else {
			return None;
		};
		let mut components = authorization.split(' ');
		let token = match (components.next(), components.next()) {
			(Some("Bearer"), Some(token)) => token.to_owned(),
			_ => return None,
		};
		Some(token)
	} else if let Some(cookies) = request.headers().get(http::header::COOKIE) {
		if let Some(name) = name {
			let Ok(cookies) = cookies.to_str() else {
				return None;
			};
			let cookies: BTreeMap<&str, &str> = match parse_cookies(cookies).collect() {
				Ok(cookies) => cookies,
				Err(_) => return None,
			};
			let token = match cookies.get(name) {
				Some(&token) => token.to_owned(),
				None => return None,
			};
			Some(token)
		} else {
			None
		}
	} else {
		None
	}
}

/// Parse an HTTP cookie string.
pub fn parse_cookies(cookies: &str) -> impl Iterator<Item = Result<(&str, &str)>> {
	cookies.split("; ").map(|cookie| {
		let mut components = cookie.split('=');
		let key = components
			.next()
			.wrap_err("Expected a key in the cookie string.")?;
		let value = components
			.next()
			.wrap_err("Expected a value in the cookie string.")?;
		Ok((key, value))
	})
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

/// 401
#[must_use]
pub fn unauthorized() -> http::Response<Outgoing> {
	http::Response::builder()
		.status(http::StatusCode::UNAUTHORIZED)
		.body(full("Unauthorized."))
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
