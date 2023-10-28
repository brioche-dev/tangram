use crate::{Incoming, Outgoing, Server, WrapErr};
use http_body_util::BodyExt;
use lmdb::Transaction;
use std::{
	os::unix::prelude::OsStrExt,
	path::{Path, PathBuf},
};
use tangram_client as tg;
use tangram_util::http::{full, not_found, ok};
use tg::{return_error, tracker::Tracker, Result, Wrap};

impl Server {
	pub async fn handle_get_tracker_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "trackers", path] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let path = PathBuf::from(
			urlencoding::decode(path)
				.wrap_err("Failed to decode the path.")?
				.as_ref(),
		);

		// Get the tracker.
		let Some(tracker) = self.try_get_tracker(&path).await? else {
			return Ok(not_found());
		};

		// Create the body.
		let body = serde_json::to_vec(&tracker).wrap_err("Failed to serialize the body.")?;

		// Create the response.
		let response = http::Response::builder()
			.status(http::StatusCode::OK)
			.body(full(body))
			.unwrap();

		Ok(response)
	}

	pub async fn handle_patch_tracker_request(
		&self,
		request: http::Request<Incoming>,
	) -> Result<http::Response<Outgoing>> {
		// Read the path params.
		let path_components: Vec<&str> = request.uri().path().split('/').skip(1).collect();
		let ["v1", "trackers", path] = path_components.as_slice() else {
			return_error!("Unexpected path.")
		};
		let path = PathBuf::from(
			urlencoding::decode(path)
				.wrap_err("Failed to decode the path.")?
				.as_ref(),
		);

		// Read the body.
		let bytes = request
			.into_body()
			.collect()
			.await
			.wrap_err("Failed to read the body.")?
			.to_bytes();
		let tracker = serde_json::from_slice(&bytes).wrap_err("Failed to deserialize the body.")?;

		self.update_tracker(&path, tracker).await?;

		Ok(ok())
	}

	pub async fn try_get_tracker(&self, path: &Path) -> Result<Option<Tracker>> {
		let txn = self
			.inner
			.database
			.env
			.begin_ro_txn()
			.wrap_err("Failed to begin the transaction.")?;
		let data = match txn.get(self.inner.database.trackers, &path.as_os_str().as_bytes()) {
			Ok(data) => data,
			Err(lmdb::Error::NotFound) => return Ok(None),
			Err(error) => return Err(error.wrap("Failed to get the tracker.")),
		};
		let tracker =
			serde_json::from_slice(data).wrap_err("Failed to deserialize the tracker.")?;
		Ok(Some(tracker))
	}

	pub async fn update_tracker(&self, _path: &Path, _tracker: Tracker) -> Result<()> {
		todo!()
	}
}

// #[derive(Debug)]
// pub struct Fsm {
// 	task: tokio::task::JoinHandle<()>,
// 	sender: tokio::sync::mpsc::Sender<PathBuf>,
// }

// pub async fn try_get_artifact_for_path(&self, path: &Path) -> Result<Option<tg::Artifact>> {
// 	let mtime = get_mtime(path).await?;
// 	let artifact = self
// 		.get_or_clear_tracker(path, mtime)?
// 		.and_then(|t| t.artifact)
// 		.map(tg::Artifact::with_id);
// 	Ok(artifact)
// }

// pub async fn set_artifact_for_path(&self, path: &Path, artifact: tg::Artifact) -> Result<()> {
// 	let artifact = artifact.id(self).await?;
// 	let mtime = get_mtime(path).await?;
// 	let tracker = self.get_or_clear_tracker(path, mtime)?.unwrap_or(Tracker {
// 		mtime,
// 		artifact: Some(artifact),
// 		package: None,
// 	});
// 	self.put_tracker(path, tracker).await
// }

// pub async fn try_get_package_for_path(&self, path: &Path) -> Result<Option<tg::Package>> {
// 	let mtime = get_mtime(path).await?;
// 	let package = self
// 		.get_or_clear_tracker(path, mtime)?
// 		.and_then(|t| t.package)
// 		.map(tg::Package::with_id);
// 	Ok(package)
// }

// pub async fn set_package_for_path(&self, path: &Path, package: tg::Package) -> Result<()> {
// 	let package = package.id(self).await?;
// 	let mtime = get_mtime(path).await?;
// 	let tracker = self.get_or_clear_tracker(path, mtime)?.unwrap_or(Tracker {
// 		mtime,
// 		artifact: None,
// 		package: Some(package),
// 	});
// 	self.put_tracker(path, tracker).await
// }

// // Attempt to retrieve a tracker for a given path. If the mtimes mistmatch, clear the tracker.
// fn get_or_clear_tracker(&self, path: &Path, mtime: u128) -> Result<Option<Tracker>> {
// 	let tracker: Option<Tracker> = {
// 		let txn = self
// 			.state
// 			.database
// 			.env
// 			.begin_ro_txn()
// 			.wrap_err("Failed to begin the transaction.")?;
// 		let key = canonicalize(path);
// 		match txn.get(self.state.database.trackers, &key) {
// 			Ok(data) => Some(serde_json::from_slice(data).wrap_err("Failed to deserialize.")?),
// 			Err(lmdb::Error::NotFound) => None,
// 			Err(e) => return Err(e.into()),
// 		}
// 	};

// 	if let Some(tracker) = tracker.as_ref() {
// 		tracing::debug!(?path, ?tracker, "Found tracker.");
// 		if tracker.mtime != mtime {
// 			tracing::debug!("mtime mismatch: clearing tracker.");
// 			self.delete_tracker(path)?;
// 			return Ok(None);
// 		}
// 	}

// 	Ok(tracker)
// }

// async fn put_tracker(&self, path: &Path, tracker: Tracker) -> Result<()> {
// 	tracing::debug!(?path, ?tracker, "Adding tracker.");
// 	self.state
// 		.fsm
// 		.read()
// 		.await
// 		.as_ref()
// 		.unwrap()
// 		.sender
// 		.send(path.into())
// 		.await
// 		.unwrap();

// 	// Add the tracker to the database
// 	{
// 		let mut txn = self
// 			.state
// 			.database
// 			.env
// 			.begin_rw_txn()
// 			.wrap_err("Failed to begin the transaction.")?;
// 		let key = canonicalize(path);
// 		let data = serde_json::to_vec(&tracker)?;
// 		txn.put(
// 			self.state.database.trackers,
// 			&key,
// 			&data,
// 			lmdb::WriteFlags::empty(),
// 		)?;
// 		txn.commit().wrap_err("Failed to commit the transaction.")?;
// 	}

// 	// Update the notifier.
// 	{
// 		let watcher = self.state.fsm.read().await;
// 		let _ = watcher.as_ref().unwrap().sender.send(path.into()).await;
// 		Ok(())
// 	}
// }

// fn delete_tracker(&self, path: &Path) -> Result<()> {
// 	tracing::debug!(?path, "Removing tracker.");
// 	let mut txn = self
// 		.state
// 		.database
// 		.env
// 		.begin_rw_txn()
// 		.wrap_err("Failed to begin the transaction.")?;
// 	let mut path = path.to_owned();
// 	loop {
// 		let key = canonicalize(path.as_ref());
// 		match txn.del(self.state.database.trackers, &key, None) {
// 			Err(error) if error == lmdb::Error::NotFound => break,
// 			Err(error) => return Err(error.wrap("Failed to delete the tracker.")),
// 			_ => (),
// 		}
// 		if !path.pop() {
// 			break;
// 		}
// 	}
// 	txn.commit().wrap_err("Failed to commit the transaction.")?;
// 	Ok(())
// }

// impl Fsm {
// 	pub(crate) fn new(server_state: Weak<State>) -> Result<Self> {
// 		use notify::Watcher;

// 		let (sender, mut receiver) = tokio::sync::mpsc::channel::<PathBuf>(128);
// 		let event_handler = move |event: notify::Result<notify::Event>| {
// 			let event = match event {
// 				Ok(event) => event,
// 				Err(e) => {
// 					tracing::warn!(?e, "Received an event error.");
// 					return;
// 				},
// 			};

// 			if !event.kind.is_modify() {
// 				return;
// 			}

// 			for path in event.paths {
// 				let Some(state) = server_state.upgrade() else {
// 					// This means that we have outlived the underlying server.
// 					return;
// 				};
// 				let server = Server { state };
// 				let _ = server.delete_tracker(&path);
// 			}
// 		};

// 		let mut watcher = notify::recommended_watcher(event_handler)
// 			.map_err(|_| error!("Failed to initialize watcher."))?;

// 		let task = tokio::task::spawn(async move {
// 			while let Some(path) = receiver.recv().await {
// 				tracing::debug!(?path, "Adding file watcher.");
// 				if let Err(e) = watcher.watch(&path, notify::RecursiveMode::Recursive) {
// 					tracing::warn!(?e, ?path, "Failed to install watcher.")
// 				}
// 			}
// 		});

// 		Ok(Self { sender, task })
// 	}
// }

// async fn get_mtime(path: &Path) -> Result<u128> {
// 	let metadata = tokio::fs::symlink_metadata(path)
// 		.await
// 		.wrap_err("Failed to get the symlink metadata.")?;
// 	let mtime = metadata
// 		.modified()
// 		.wrap_err("Failed to get the last modification time.")?
// 		.duration_since(std::time::UNIX_EPOCH)
// 		.unwrap()
// 		.as_micros();
// 	Ok(mtime)
// }

// fn canonicalize(path: &Path) -> &'_ [u8] {
// 	let path = path.as_os_str().as_bytes();
// 	if path.ends_with(b"/") {
// 		&path[0..path.len() - 1]
// 	} else {
// 		path
// 	}
// }

// fn delete_directory_trackers(env: &lmdb::Environment, trackers: lmdb::Database) -> Result<()> {
// 	let paths = {
// 		let txn = env
// 			.begin_ro_txn()
// 			.wrap_err("Failed to begin the transaction.")?;
// 		let mut cursor = txn
// 			.open_ro_cursor(trackers)
// 			.wrap_err("Failed to open the cursor.")?;
// 		cursor
// 			.iter()
// 			.filter_map(|entry| {
// 				let (path, _) = entry.ok()?;
// 				let path = PathBuf::from(OsStr::from_bytes(path));
// 				path.is_dir().then_some(path)
// 			})
// 			.collect::<Vec<_>>()
// 	};

// 	let mut txn = env
// 		.begin_rw_txn()
// 		.wrap_err("Failed to begin the transaction.")?;
// 	for path in paths {
// 		let key = path.as_os_str().as_bytes();
// 		let _ = txn.del(trackers, &key, None);
// 	}
// 	txn.commit().wrap_err("Failed to commit the transaction.")?;
// 	Ok(())
// }

// impl Drop for Fsm {
// 	fn drop(&mut self) {
// 		self.task.abort();
// 	}
// }
