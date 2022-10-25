use crate::{builder::Builder, js};
use anyhow::{anyhow, bail, Context, Result};
use futures::future::try_join_all;
use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	sync::Arc,
};
use tokio::sync::RwLock;

pub mod load;
pub mod resolve;
pub mod runtime;
pub mod transpile;
pub mod url;

use runtime::{CheckRequest, Envelope, Request, Response};

use self::runtime::GetDiagnosticsRequest;

#[derive(Clone)]
pub struct Compiler {
	state: Arc<State>,
}

pub struct State {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
	sender: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<Option<Envelope>>>>,
	open_files: RwLock<HashMap<PathBuf, OpenFile, fnv::FnvBuildHasher>>,
	// saved_files: RwLock<HashMap<PathBuf,
}

struct OpenFile {
	version: i32,
	source: String,
}

impl Compiler {
	#[must_use]
	pub fn new(builder: Builder, main_runtime_handle: tokio::runtime::Handle) -> Compiler {
		let state = State {
			builder,
			main_runtime_handle,
			sender: std::sync::Mutex::new(None),
			open_files: RwLock::new(HashMap::default()),
		};
		Compiler {
			state: Arc::new(state),
		}
	}

	fn runtime_sender(&self) -> tokio::sync::mpsc::UnboundedSender<Option<Envelope>> {
		let mut lock = self.state.sender.lock().unwrap();
		if let Some(sender) = lock.as_ref() {
			sender.clone()
		} else {
			// Create a channel to send requests to the compiler runtime.
			let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Option<Envelope>>();

			// Spawn a thread for the compiler runtime to respond to requests.
			std::thread::spawn({
				let compiler = self.clone();
				let main_runtime_handle = self.state.main_runtime_handle.clone();
				move || {
					let runtime = tokio::runtime::Builder::new_current_thread()
						.enable_all()
						.build()
						.unwrap();
					runtime.block_on(async move {
						let mut runtime = runtime::Runtime::new(compiler, main_runtime_handle);
						while let Some(envelope) = receiver.recv().await {
							// If the received value is `None`, then the thread should terminate.
							let envelope = if let Some(envelope) = envelope {
								envelope
							} else {
								break;
							};

							// Handle the request.
							let response = runtime.handle(envelope.request).await;

							// Send the response.
							envelope.sender.send(response).ok();
						}
					});
				}
			});

			// Save the sender.
			lock.replace(sender.clone());

			sender
		}
	}

	/// Send an `Request` into the runtime for evaluation.
	async fn request(&self, request: Request) -> Result<Response> {
		// Create a channel for the compiler runtime to send responses.
		let (sender, receiver) = tokio::sync::oneshot::channel();

		// Send the request.
		let envelope = Envelope { request, sender };
		self.runtime_sender()
			.send(Some(envelope))
			.map_err(|_| anyhow!("Failed to send the request."))?;

		// Receive the response.
		let response = receiver
			.await
			.context("Failed to receive a response for the request.")?
			.context("The handler errored.")?;

		Ok(response)
	}
}

impl Compiler {
	pub async fn get_version(&self, url: &js::Url) -> Result<i32> {
		match url {
			js::Url::PathModule {
				package_path,
				sub_path,
			} => {
				let path = package_path.join(sub_path);
				let path = tokio::fs::canonicalize(&path).await?;
				let open_files = self.state.open_files.read().await;
				if let Some(open_file) = open_files.get(&path) {
					Ok(open_file.version)
				} else {
					Ok(0)
				}
			},

			js::Url::PathTargets { .. } => Ok(0),

			// Package module and package targets URLs have hashes, so they never change. Similarly, the typescript lib.d.ts never changes.
			js::Url::PackageModule { .. } | js::Url::PackageTargets { .. } | js::Url::TsLib => {
				Ok(0)
			},
		}
	}
}

impl Compiler {
	pub async fn open_document(&self, path: &Path, version: i32, source: String) {
		let document = OpenFile { version, source };
		self.state
			.open_files
			.write()
			.await
			.insert(path.to_owned(), document);
	}

	pub async fn close_document(&self, path: &Path) {
		self.state.open_files.write().await.remove(path);
	}

	pub async fn change_document(&self, path: &Path, version: i32, source: String) {
		let document = OpenFile { version, source };
		self.state
			.open_files
			.write()
			.await
			.insert(path.to_owned(), document);
	}
}

impl Compiler {
	/// Get all diagnostics for a package.
	pub async fn check(&self, urls: Vec<js::Url>) -> Result<BTreeMap<js::Url, Vec<Diagnostic>>> {
		let paths = urls
			.into_iter()
			.map(|url| url.to_typescript_path())
			.collect();

		// Create the request.
		let request = Request::Check(CheckRequest { paths });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Check(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let diagnostics = try_join_all(response.diagnostics.into_iter().map(
			|(path, diagnostics)| async move {
				let url = js::Url::from_typescript_path(&path).await?;
				Ok::<_, anyhow::Error>((url, diagnostics))
			},
		))
		.await?
		.into_iter()
		.collect();

		Ok(diagnostics)
	}

	pub async fn get_diagnostics(&self) -> Result<BTreeMap<js::Url, Vec<Diagnostic>>> {
		// Create the request.
		let request = Request::GetDiagnostics(GetDiagnosticsRequest {});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::GetDiagnostics(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let diagnostics = try_join_all(response.diagnostics.into_iter().map(
			|(path, diagnostics)| async move {
				let url = js::Url::from_typescript_path(&path).await?;
				Ok::<_, anyhow::Error>((url, diagnostics))
			},
		))
		.await?
		.into_iter()
		.collect();

		Ok(diagnostics)
	}
}

impl Drop for Compiler {
	fn drop(&mut self) {
		if let Some(sender) = self.state.sender.lock().unwrap().take() {
			sender.send(None).ok();
		}
	}
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Diagnostic {
	pub location: Option<DiagnosticLocation>,
	pub message: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DiagnosticLocation {
	pub path: String,
	pub range: Range,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Range {
	pub start: Position,
	pub end: Position,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Position {
	pub line: u32,
	pub character: u32,
}
