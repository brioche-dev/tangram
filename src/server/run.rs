use crate::{run, system, task, Blob, Client, Error, Result, Run, Server, Task, Value, WrapErr};
use futures::{
	stream::{self, BoxStream},
	StreamExt, TryStreamExt,
};
use lmdb::Transaction;
use std::sync::Arc;
use tokio_util::io::StreamReader;

impl Server {
	/// Attempt to get the run for a task.
	pub(crate) async fn try_get_run_for_task(&self, id: task::Id) -> Result<Option<run::Id>> {
		// Attempt to get the run for the task from the running state.
		if let Some(run_id) = self.state.running.read().unwrap().0.get(&id).copied() {
			return Ok(Some(run_id));
		}

		// Attempt to get the run for the task from the database.
		if let Some(run_id) = self.try_get_run_for_task_from_database(id)? {
			return Ok(Some(run_id));
		}

		// Attempt to get the run for the task from the parent.
		if let Some(run_id) = self.try_get_run_for_task_from_parent(id).await? {
			return Ok(Some(run_id));
		}

		Ok(None)
	}

	/// Attempt to get the run for the task from the database.
	fn try_get_run_for_task_from_database(&self, id: task::Id) -> Result<Option<run::Id>> {
		// Get the run for the task from the database.
		let txn = self.state.database.env.begin_ro_txn()?;
		match txn.get(self.state.database.assignments, &id.as_bytes()) {
			Ok(run_id) => Ok(Some(run_id.try_into().wrap_err("Invalid ID.")?)),
			Err(lmdb::Error::NotFound) => Ok(None),
			Err(error) => Err(error.into()),
		}
	}

	/// Attempt to get the run for the task from the parent.
	async fn try_get_run_for_task_from_parent(&self, id: task::Id) -> Result<Option<run::Id>> {
		// Get the parent.
		let Some(parent) = self.state.parent.as_ref() else {
			return Ok(None);
		};

		// Get the assignment.
		let Some(run_id) = parent.try_get_run_for_task(id).await? else {
			return Ok(None);
		};

		Ok(Some(run_id))
	}

	/// Get or create a run for a task.
	pub(crate) async fn get_or_create_run_for_task(&self, task_id: task::Id) -> Result<run::Id> {
		// Attempt to get the run for the task.
		if let Some(run_id) = self.try_get_run_for_task(task_id).await? {
			return Ok(run_id);
		}

		// Otherwise, create a new run and add it to the server's state.
		let run_id = run::Id::new();
		let state = Arc::new(run::State::new()?);
		{
			let mut running = self.state.running.write().unwrap();
			running.0.insert(task_id, run_id);
			running.1.insert(run_id, state.clone());
		}

		// Spawn the task.
		tokio::spawn({
			let task = Task::with_id(task_id);
			let server = self.clone();
			async move {
				let client = &Client::with_server(server.clone());

				let object = task.object(client).await?;

				let output = match object.host.os() {
					system::Os::Js => server.run_js(&task, &state).await?,
					_ => todo!(),
				};

				// Set the result on the state.
				state.set_output(output).await;

				// Create the object.
				let children = state.children().try_collect().await?;
				let log = StreamReader::new(
					state
						.log()
						.await?
						.map_ok(::bytes::Bytes::from)
						.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error)),
				);
				let log = Blob::with_reader(client, log).await?;
				let output = state.output().await;
				let object = run::Object {
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
					.try_put_object_bytes(run_id.into(), &bytes)
					.await
					.wrap_err("Failed to put the object.")?
					.ok()
					.wrap_err("Expected all children to be stored.")?;

				// Create a write transaction.
				let mut txn = server.state.database.env.begin_rw_txn()?;

				// Set the run for the task in the database.
				txn.put(
					server.state.database.assignments,
					&task_id.as_bytes(),
					&run_id.as_bytes(),
					lmdb::WriteFlags::empty(),
				)?;

				// Commit the transaction.
				txn.commit()?;

				// Remove the run from the running state.
				server.state.running.write().unwrap().1.remove(&run_id);

				Ok::<_, Error>(())
			}
		});

		Ok(run_id)
	}

	pub(crate) async fn try_get_run_children(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, Result<run::Id>>>> {
		let client = &Client::with_server(self.clone());
		let run = Run::with_id(id);

		// Attempt to stream the children from the running state.
		let state = self.state.running.read().unwrap().1.get(&run.id()).cloned();
		if let Some(state) = state {
			let children = state.children();
			return Ok(Some(children.map_ok(|child| child.id()).boxed()));
		}

		// Attempt to get the children from the object.
		'a: {
			let Some(object) = run.try_get_object(client).await? else {
				break 'a;
			};
			return Ok(Some(
				stream::iter(object.children.clone())
					.map(|child| child.id())
					.map(Ok)
					.boxed(),
			));
		}

		// Attempt to stream the children from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(children) = parent.try_get_run_children(id).await? else {
				break 'a;
			};
			return Ok(Some(children));
		}

		Ok(None)
	}

	pub(crate) async fn try_get_run_log(
		&self,
		id: run::Id,
	) -> Result<Option<BoxStream<'static, Result<Vec<u8>>>>> {
		let client = &Client::with_server(self.clone());
		let run = Run::with_id(id);

		// Attempt to stream the log from the running state.
		let state = self.state.running.read().unwrap().1.get(&run.id()).cloned();
		if let Some(state) = state {
			let log = state.log().await?;
			return Ok(Some(log));
		}

		// Attempt to get the log from the object.
		'a: {
			let Some(object) = run.try_get_object(client).await? else {
				break 'a;
			};
			let object = object.clone();
			let client = client.clone();
			return Ok(Some(
				stream::once(async move { object.log.bytes(&client).await }).boxed(),
			));
		}

		// Attempt to stream the log from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(log) = parent.try_get_run_log(id).await? else {
				break 'a;
			};
			return Ok(Some(log));
		}

		Ok(None)
	}

	pub(crate) async fn try_get_run_output(&self, id: run::Id) -> Result<Option<Option<Value>>> {
		let client = &Client::with_server(self.clone());
		let run = Run::with_id(id);

		// Attempt to await the output from the running state.
		let state = self.state.running.read().unwrap().1.get(&run.id()).cloned();
		if let Some(state) = state {
			let output = state.output().await;
			return Ok(Some(output));
		}

		// Attempt to get the output from the object.
		'a: {
			let Some(object) = run.try_get_object(client).await? else {
				break 'a;
			};
			return Ok(Some(object.output.clone()));
		}

		// Attempt to await the output from the parent.
		'a: {
			let Some(parent) = self.state.parent.as_ref() else {
				break 'a;
			};
			let Some(output) = parent.try_get_run_output(id).await? else {
				break 'a;
			};
			return Ok(Some(output));
		}

		Ok(None)
	}
}
