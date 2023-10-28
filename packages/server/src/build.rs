use super::Server;
use crate::Progress;
use bytes::Bytes;
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use http_body_util::StreamBody;
use lmdb::Transaction;
use tangram_client as tg;
use tangram_util::http::{full, not_found, Incoming, Outgoing};
use tg::{build, return_error, system, target, Build, Result, Target, Value, Wrap, WrapErr};

impl Server {
	pub async fn handle_try_get_build_for_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
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

	pub async fn handle_get_or_create_build_for_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "targets", id, "build"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Get or create the build for the target.
		let build_id = self.get_or_create_build_for_target(&id).await?;

		// Create the response.
		let body = serde_json::to_vec(&build_id).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder().body(full(body)).unwrap();
		Ok(response)
	}

	pub async fn handle_try_get_build_target_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
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

	pub async fn handle_try_get_build_children_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
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

	pub async fn handle_get_build_log_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "log"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the log.
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

	pub async fn handle_try_get_build_result_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<hyper::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let [_, "builds", id, "result"] = path_components.as_slice() else {
			return_error!("Unexpected path.");
		};
		let id = id.parse().wrap_err("Failed to parse the ID.")?;

		// Attempt to get the result.
		let Some(result) = self.try_get_build_result(&id).await? else {
			return Ok(not_found());
		};

		// Create the response.
		let result = match result {
			Ok(value) => Ok(value.data(self).await?),
			Err(error) => Err(error),
		};
		let body = serde_json::to_vec(&result).wrap_err("Failed to serialize the response.")?;
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();
		Ok(response)
	}

	/// Attempt to get the build for a target.
	pub async fn try_get_build_for_target(&self, id: &target::Id) -> Result<Option<build::Id>> {
		// Attempt to get the build for the target from the running state.
		if let Some(build_id) = self.inner.builds.read().unwrap().0.get(id).cloned() {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the database.
		if let Some(build_id) = self.try_get_build_for_target_from_database(id)? {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the parent.
		if let Ok(Some(build_id)) = self.try_get_build_for_target_from_parent(id).await {
			return Ok(Some(build_id));
		}

		Ok(None)
	}

	/// Attempt to get the build for the target from the database.
	fn try_get_build_for_target_from_database(&self, id: &target::Id) -> Result<Option<build::Id>> {
		// Get the build for the target from the database.
		let txn = self
			.inner
			.database
			.env
			.begin_ro_txn()
			.wrap_err("Failed to begin the transaction.")?;
		match txn.get(self.inner.database.assignments, &id.to_bytes()) {
			Ok(build_id) => Ok(Some(build_id.try_into().wrap_err("Invalid ID.")?)),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.wrap("Failed to get the assignment.")),
		}
	}

	/// Attempt to get the build for the target from the parent.
	async fn try_get_build_for_target_from_parent(
		&self,
		id: &target::Id,
	) -> Result<Option<build::Id>> {
		// Get the parent.
		let Some(parent) = self.inner.parent.as_ref() else {
			return Ok(None);
		};

		// Get the build for the target from the parent.
		let Some(build_id) = parent.try_get_build_for_target(id).await? else {
			return Ok(None);
		};

		Ok(Some(build_id))
	}

	/// Get or create a build for a target.
	pub async fn get_or_create_build_for_target(
		&self,
		target_id: &target::Id,
	) -> Result<build::Id> {
		let target = Target::with_id(target_id.clone());

		// Attempt to get the build for the target.
		if let Some(build_id) = self.try_get_build_for_target(target_id).await? {
			return Ok(build_id);
		}

		// Otherwise, create a new build and add its progress to the server.
		let build_id = build::Id::new();
		let progress = Progress::new(build_id.clone(), target)?;
		{
			let mut running = self.inner.builds.write().unwrap();
			running.0.insert(target_id.clone(), build_id.clone());
			running.1.insert(build_id.clone(), progress.clone());
		}

		// Spawn the task.
		tokio::spawn({
			let server = self.clone();
			let target_id = target_id.clone();
			let build_id = build_id.clone();
			async move { server.build_inner(target_id, build_id, progress).await }
		});

		Ok(build_id)
	}

	async fn build_inner(
		&self,
		target_id: target::Id,
		build_id: build::Id,
		progress: Progress,
	) -> Result<()> {
		// Build the target.
		let target = Target::with_id(target_id.clone());
		match target.host(self).await?.os() {
			system::Os::Js => {
				// Build the target on the server's local pool because it is a `!Send` future.
				self.inner
					.local_pool
					.spawn_pinned({
						let server = self.clone();
						let target = target.clone();
						let progress = progress.clone();
						let main_runtime_handle = tokio::runtime::Handle::current();
						move || async move {
							tangram_runtime::js::run(
								&server,
								target,
								&progress,
								main_runtime_handle,
							)
							.await
						}
					})
					.await
					.wrap_err("Failed to join the task.")?;
			},
			system::Os::Darwin => {
				#[cfg(target_os = "macos")]
				tangram_runtime::darwin::run(self, target, &progress).await;
				#[cfg(not(target_os = "macos"))]
				return_error!("Cannot run a darwin build on this host.");
			},
			system::Os::Linux => {
				#[cfg(target_os = "linux")]
				tangram_runtime::linux::run(self, target, &progress).await;
				#[cfg(not(target_os = "linux"))]
				return_error!("Cannot run a linux build on this host.");
			},
		};

		// Finish the build.
		let _build = progress.finish(self).await?;

		// Create a write transaction.
		let mut txn = self
			.inner
			.database
			.env
			.begin_rw_txn()
			.wrap_err("Failed to begin the transaction.")?;

		// Set the build for the target.
		txn.put(
			self.inner.database.assignments,
			&target_id.to_bytes(),
			&build_id.to_bytes(),
			lmdb::WriteFlags::empty(),
		)
		.wrap_err("Failed to store the item.")?;

		// Commit the transaction.
		txn.commit().wrap_err("Failed to commit the transaction.")?;

		// Remove the state of the running build.
		self.inner.builds.write().unwrap().1.remove(&build_id);

		Ok(())
	}

	pub async fn try_get_build_target(&self, id: &tg::build::Id) -> Result<Option<tg::target::Id>> {
		// Attempt to get the target from the running state.
		let state = self.inner.builds.read().unwrap().1.get(id).cloned();
		if let Some(state) = state {
			return Ok(Some(state.target().id(self).await?.clone()));
		}

		// Attempt to get the target from the object.
		'a: {
			let build = Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.target.expect_id().clone()));
		}

		Ok(None)
	}

	pub async fn try_get_build_children(
		&self,
		id: &build::Id,
	) -> Result<Option<BoxStream<'static, Result<build::Id>>>> {
		// Attempt to stream the children from the running state.
		let state = self.inner.builds.read().unwrap().1.get(id).cloned();
		if let Some(state) = state {
			let children = state.children_stream();
			return Ok(Some(children.map_ok(|child| child.id().clone()).boxed()));
		}

		// Attempt to get the children from the object.
		'a: {
			let build = Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(
				stream::iter(object.children.clone())
					.map(|child| child.id().clone())
					.map(Ok)
					.boxed(),
			));
		}

		// Attempt to stream the children from the parent.
		'a: {
			let Some(parent) = self.inner.parent.as_ref() else {
				break 'a;
			};
			let Some(children) = parent.try_get_build_children(id).await? else {
				break 'a;
			};
			return Ok(Some(children));
		}

		Ok(None)
	}

	pub async fn try_get_build_log(
		&self,
		id: &build::Id,
	) -> Result<Option<BoxStream<'static, Result<Bytes>>>> {
		// Attempt to stream the log from the running state.
		let state = self.inner.builds.read().unwrap().1.get(id).cloned();
		if let Some(state) = state {
			let log = state.log_stream().await?;
			return Ok(Some(log));
		}

		// Attempt to get the log from the object.
		'a: {
			let build = Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			let bytes = object.log.bytes(self).await?;
			return Ok(Some(stream::once(async move { Ok(bytes.into()) }).boxed()));
		}

		// Attempt to stream the log from the parent.
		'a: {
			let Some(parent) = self.inner.parent.as_ref() else {
				break 'a;
			};
			let Some(log) = parent.try_get_build_log(id).await? else {
				break 'a;
			};
			return Ok(Some(log));
		}

		Ok(None)
	}

	pub async fn try_get_build_result(&self, id: &build::Id) -> Result<Option<Result<Value>>> {
		// Attempt to await the result from the running state.
		let state = self.inner.builds.read().unwrap().1.get(id).cloned();
		if let Some(state) = state {
			let result = state.wait_for_result().await;
			return Ok(Some(result));
		}

		// Attempt to get the result from the object.
		'a: {
			let build = Build::with_id(id.clone());
			let Some(object) = build.try_get_object(self).await? else {
				break 'a;
			};
			return Ok(Some(object.result.clone()));
		}

		// Attempt to await the result from the parent.
		'a: {
			let Some(parent) = self.inner.parent.as_ref() else {
				break 'a;
			};
			let Some(result) = parent.try_get_build_result(id).await? else {
				break 'a;
			};
			return Ok(Some(result));
		}

		Ok(None)
	}
}
