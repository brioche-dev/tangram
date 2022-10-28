use self::runtime::{
	CheckRequest, Envelope, GetDiagnosticsRequest, GetHoverRequest, GotoDefintionRequest,
	QuickInfo, Request, Response,
};
use crate::{builder::Builder, js};
use anyhow::{anyhow, bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	sync::Arc,
	time::SystemTime,
};
use tokio::sync::RwLock;

pub mod load;
pub mod resolve;
pub mod runtime;
pub mod transpile;
pub mod url;

#[derive(Clone)]
pub struct Compiler {
	state: Arc<State>,
}

pub struct State {
	builder: Builder,
	sender: std::sync::Mutex<Option<tokio::sync::mpsc::UnboundedSender<Option<Envelope>>>>,
	files: RwLock<HashMap<PathBuf, File, fnv::FnvBuildHasher>>,
}

enum File {
	Opened(OpenedFile),
	Unopened(UnopenedFile),
}

struct OpenedFile {
	url: js::Url,
	version: i32,
	text: String,
}

struct UnopenedFile {
	_url: js::Url,
	version: i32,
	modified: SystemTime,
}

impl Compiler {
	#[must_use]
	pub fn new(builder: Builder) -> Compiler {
		let state = State {
			builder,
			sender: std::sync::Mutex::new(None),
			files: RwLock::new(HashMap::default()),
		};
		Compiler {
			state: Arc::new(state),
		}
	}

	fn runtime_sender(&self) -> tokio::sync::mpsc::UnboundedSender<Option<Envelope>> {
		let main_runtime_handle = tokio::runtime::Handle::current();
		let mut lock = self.state.sender.lock().unwrap();
		if let Some(sender) = lock.as_ref() {
			sender.clone()
		} else {
			// Create a channel to send requests to the compiler runtime.
			let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Option<Envelope>>();

			// Spawn a thread for the compiler runtime to respond to requests.
			std::thread::spawn({
				let compiler = self.clone();
				move || {
					let mut runtime = runtime::Runtime::new(compiler, main_runtime_handle);
					while let Some(envelope) = receiver.blocking_recv() {
						// If the received value is `None`, then the thread should terminate.
						let envelope = if let Some(envelope) = envelope {
							envelope
						} else {
							break;
						};

						// Handle the request.
						let response = runtime.handle(envelope.request);

						// Send the response.
						envelope.sender.send(response).ok();
					}
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
		// Get the path for the URL, or return version 0 for URLs whose contents never change.
		let path = match url {
			// Path modules update versions when the file at their path changes.
			js::Url::PathModule {
				package_path,
				module_path,
			} => package_path.join(module_path),

			// Path targets update versions when their manifest changes.
			js::Url::PathTargets { package_path } => package_path.join("tangram.json"),

			// Package module and package targets URLs have hashes. They never change, so we can always return 0. The same goes for the typescript libraries.
			js::Url::PackageModule { .. }
			| js::Url::PackageTargets { .. }
			| js::Url::TsLib { .. } => {
				return Ok(0);
			},
		};

		let mut files = self.state.files.write().await;
		match files.get_mut(&path) {
			// If the file is not in the files map, add it at version 0 and save its modified time.
			None => {
				let metadata = tokio::fs::metadata(&path).await?;
				let modified = metadata.modified()?;
				files.insert(
					path,
					File::Unopened(UnopenedFile {
						_url: url.clone(),
						version: 0,
						modified,
					}),
				);
				Ok(0)
			},

			// If the file is opened, return its version.
			Some(File::Opened(opened_file)) => Ok(opened_file.version),

			// If the file is in the files map but unopened, update its version if the file's modified time is newer, and return the version.
			Some(File::Unopened(unopened_file)) => {
				let metadata = tokio::fs::metadata(&path).await?;
				let modified = metadata.modified()?;
				if modified > unopened_file.modified {
					unopened_file.modified = modified;
					unopened_file.version += 1;
				}
				Ok(unopened_file.version)
			},
		}
	}
}

impl Compiler {
	pub async fn open_file(&self, path: &Path, version: i32, text: String) {
		let url = js::Url::for_path(path).await.unwrap();
		let file = File::Opened(OpenedFile { url, version, text });
		self.state.files.write().await.insert(path.to_owned(), file);
	}

	pub async fn close_file(&self, path: &Path) {
		self.state.files.write().await.remove(path);
	}

	pub async fn change_file(&self, path: &Path, version: i32, text: String) {
		let url = js::Url::for_path(path).await.unwrap();
		let file = File::Opened(OpenedFile { url, version, text });
		self.state.files.write().await.insert(path.to_owned(), file);
	}
}

impl Compiler {
	/// Get all diagnostics for a package.
	pub async fn check(&self, urls: Vec<js::Url>) -> Result<BTreeMap<js::Url, Vec<Diagnostic>>> {
		// Create the request.
		let request = Request::Check(CheckRequest { urls });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Check(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let diagnostics = response.diagnostics;

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
		let diagnostics = response.diagnostics;

		Ok(diagnostics)
	}

	pub async fn hover(&self, url: js::Url, position: Position) -> Result<Option<QuickInfo>> {
		// Create the request.
		let request = Request::GetHover(GetHoverRequest { url, position });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::GetHover(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let info = response.info;

		Ok(info)
	}

	pub async fn goto_definition(
		&self,
		url: js::Url,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::GotoDefinition(GotoDefintionRequest { url, position });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::GotoDefinition(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let locations = response.locations;

		Ok(locations)
	}

	pub fn virtual_text_document(&self, path: &Utf8Path) -> Result<Option<String>> {
		// Get the contents for this document.
		let contents = self.load_ts_lib(path)?;
		Ok(Some(contents))
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
	pub location: Option<Location>,
	pub message: String,
	pub category: DiagnosticCategory,
}

#[derive(Debug, Clone)]
pub enum DiagnosticCategory {
	Warning,
	Error,
	Suggestion,
	Message,
}

impl<'de> serde::Deserialize<'de> for DiagnosticCategory {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let s: i64 = serde::Deserialize::deserialize(deserializer)?;
		Ok(DiagnosticCategory::from(s))
	}
}

impl From<i64> for DiagnosticCategory {
	fn from(value: i64) -> Self {
		match value {
			0 => DiagnosticCategory::Warning,
			1 => DiagnosticCategory::Error,
			2 => DiagnosticCategory::Suggestion,
			3 => DiagnosticCategory::Message,
			_ => panic!("Unknown value: {}", value),
		}
	}
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Location {
	pub url: js::Url,
	pub range: Range,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Range {
	pub start: Position,
	pub end: Position,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Position {
	pub line: u32,
	pub character: u32,
}

pub struct VirtualTextDocumentParams {
	pub path: Utf8PathBuf,
}
