use crate::{
	build, system, target, Blob, Build, Client, Error, Result, Server, Target, Value, WrapErr,
};
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use lmdb::Transaction;
use std::sync::Arc;
use tokio_util::io::StreamReader;

impl Server {
	/// Attempt to get the build for a target.
	pub(crate) async fn try_get_build_for_target(
		&self,
		id: target::Id,
	) -> Result<Option<build::Id>> {
		// Attempt to get the build for the target from the running state.
		if let Some(build_id) = self.state.running.read().unwrap().0.get(&id).copied() {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the database.
		if let Some(build_id) = self.try_get_build_for_target_from_database(id)? {
			return Ok(Some(build_id));
		}

		// Attempt to get the build for the target from the parent.
		if let Some(build_id) = self.try_get_build_for_target_from_parent(id).await? {
			return Ok(Some(build_id));
		}

		Ok(None)
	}

	/// Attempt to get the build for the target from the database.
	fn try_get_build_for_target_from_database(&self, id: target::Id) -> Result<Option<build::Id>> {
		// Get the build for the target from the database.
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.assignments, &id.as_bytes()) {
			Ok(build_id) => Ok(Some(build_id.try_into().wrap_err("Invalid ID.")?)),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	/// Attempt to get the build for the target from the parent.
	async fn try_get_build_for_target_from_parent(
		&self,
		id: target::Id,
	) -> Result<Option<build::Id>> {
		// Get the parent.
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the assignment.
		let Some(build_id) = parent.try_get_build_for_target(id).await? else {
			return Ok(None);
		};

		Ok(Some(build_id))
	}

	/// Get or create a build for a target.
	pub(crate) async fn get_or_create_build_for_target(
		&self,
		target_id: target::Id,
	) -> Result<build::Id> {
		// Attempt to get the build for the target.
		if let Some(build_id) = self.try_get_build_for_target(target_id).await? {
			return Ok(build_id);
		}

		// Otherwise, create a new build and add it to the server's state.
		let build_id = build::Id::new();
		let state = Arc::new(build::State::new()?);
		{
			let mut running = self.state.running.write().unwrap();
			running.0.insert(target_id, build_id);
			running.1.insert(build_id, state.clone());
		}

		// Spawn the task.
		tokio::spawn({
			let target = Target::with_id(target_id);
			let server = self.clone();
			async move {
				let client = &Client::with_server(server.clone());

				let object = target.object(client).await?;

				let output = match object.host.os() {
					system::Os::Js => {
						// Build the target on the server's local pool because it is a `!Send` future.
						server
							.state
							.local_pool
							.spawn_pinned({
								let client = client.clone();
								let target = target.clone();
								let state = state.clone();
								let main_runtime_handle = tokio::runtime::Handle::current();
								move || async move {
									crate::build::js::run(
										client.clone(),
										target,
										state,
										main_runtime_handle,
									)
									.await
								}
							})
							.await
							.wrap_err("Failed to join the task.")?
					},
					_ => todo!(),
				};

				// Set the result on the state.
				state.set_output(output).await;

				// Create the object.
				let children = state.children().collect().await;
				let log = StreamReader::new(
					state
						.log()
						.await?
						.map(::bytes::Bytes::from)
						.map(Ok::<_, std::io::Error>),
				);
				let log = Blob::with_reader(client, log).await?;
				let output = state.output().await;
				let object = build::Object {
					children,
					log,
					output,
				};

				// Store the children.
				object
					.children()
					.into_iter()
					.map(|child| async move { child.store(client).await })
					.collect::<futures::stream::FuturesUnordered<_>>()
					.try_collect()
					.await?;

				// Get the data.
				let data = object.to_data();

				// Serialize the data.
				let bytes = data.serialize()?;

				// Store the object.
				client
					.try_put_object_bytes(build_id.into(), &bytes)
					.await
					.wrap_err("Failed to put the object.")?
					.ok()
					.wrap_err("Expected all children to be stored.")?;

				// Create a write transaction.
				let mut txn = server.state.database.env.begin_rw_txn()?;

				// Set the build for the target in the database.
				txn.put(
					server.state.database.assignments,
					&target_id.as_bytes(),
					&build_id.as_bytes(),
					lmdb::WriteFlags::empty(),
				)?;

				// Commit the transaction.
				txn.commit()?;

				// Remove the build from the running state.
				server.state.running.write().unwrap().1.remove(&build_id);

				Ok::<_, Error>(())
			}
		});

		Ok(build_id)
	}

	pub(crate) async fn try_get_build_children(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, build::Id>>> {
		let client = &Client::with_server(self.clone());
		let build = Build::with_id(id);

		// Attempt to stream the children from the running state.
		let state = self
			.state
			.running
			.read()
			.unwrap()
			.1
			.get(&build.id())
			.cloned();
		if let Some(state) = state {
			let children = state.children();
			return Ok(Some(children.map(|child| child.id()).boxed()));
		}

		// Attempt to get the children from the object.
		'a: {
			let Some(object) = build.try_get_object(client).await? else {
				break 'a;
			};
			return Ok(Some(
				stream::iter(object.children.clone())
					.map(|child| child.id())
					.boxed(),
			));
		}

		// Attempt to stream the children from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(children) = parent.try_get_build_children(id).await? else {
				break 'a;
			};
			return Ok(Some(children));
		}

		Ok(None)
	}

	pub(crate) async fn try_get_build_log(
		&self,
		id: build::Id,
	) -> Result<Option<BoxStream<'static, Vec<u8>>>> {
		let client = &Client::with_server(self.clone());
		let build = Build::with_id(id);

		// Attempt to stream the log from the running state.
		let state = self
			.state
			.running
			.read()
			.unwrap()
			.1
			.get(&build.id())
			.cloned();
		if let Some(state) = state {
			let log = state.log().await?;
			return Ok(Some(log));
		}

		// Attempt to get the log from the object.
		'a: {
			let Some(object) = build.try_get_object(client).await? else {
				break 'a;
			};
			let object = object.clone();
			let client = client.clone();
			let bytes = object.log.bytes(&client).await?;
			return Ok(Some(stream::once(async move { bytes }).boxed()));
		}

		// Attempt to stream the log from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(log) = parent.try_get_build_log(id).await? else {
				break 'a;
			};
			return Ok(Some(log));
		}

		Ok(None)
	}

	pub(crate) async fn try_get_build_output(
		&self,
		id: build::Id,
	) -> Result<Option<Option<Value>>> {
		let client = &Client::with_server(self.clone());
		let build = Build::with_id(id);

		// Attempt to await the output from the running state.
		let state = self
			.state
			.running
			.read()
			.unwrap()
			.1
			.get(&build.id())
			.cloned();
		if let Some(state) = state {
			let output = state.output().await;
			return Ok(Some(output));
		}

		// Attempt to get the output from the object.
		'a: {
			let Some(object) = build.try_get_object(client).await? else {
				break 'a;
			};
			return Ok(Some(object.output.clone()));
		}

		// Attempt to await the output from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(output) = parent.try_get_build_output(id).await? else {
				break 'a;
			};
			return Ok(Some(output));
		}

		Ok(None)
	}
}
