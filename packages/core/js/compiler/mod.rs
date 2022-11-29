use self::{
	runtime::types::{
		CheckRequest, CompletionRequest, FindRenameLocationsRequest, FindRenameLocationsResponse,
		GetDiagnosticsRequest, GetDiagnosticsResponse, GetHoverRequest, GetReferencesRequest,
		GetReferencesResponse, GotoDefinitionResponse, GotoDefintionRequest, Request, Response,
		TranspileRequest,
	},
	types::{CompletionEntry, Diagnostic, Location, Position, Range, TranspileOutput},
};
use crate::{builder::Builder, js};
use anyhow::{anyhow, bail, Context, Result};
use nom::ToUsize;
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
pub mod types;
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

#[derive(Debug)]
enum File {
	Opened(OpenedFile),
	Unopened(UnopenedFile),
}

#[derive(Debug)]
struct OpenedFile {
	url: js::Url,
	version: i32,
	text: String,
}

#[derive(Debug)]
struct UnopenedFile {
	_url: js::Url,
	version: i32,
	modified: SystemTime,
}

pub struct Envelope {
	pub request: Request,
	pub sender: tokio::sync::oneshot::Sender<Result<Response>>,
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

			// Package module and package targets URLs have hashes. They never change, so we can always return 0. The same goes for the libs.
			js::Url::Lib { .. }
			| js::Url::PackageModule { .. }
			| js::Url::PackageTargets { .. } => {
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
		let Ok(url) = js::Url::for_path(path).await else { return };
		let file = File::Opened(OpenedFile { url, version, text });
		self.state.files.write().await.insert(path.to_owned(), file);
	}

	pub async fn close_file(&self, path: &Path) {
		self.state.files.write().await.remove(path);
	}

	pub async fn change_file(&self, path: &Path, version: i32, range: Option<Range>, text: String) {
		let Some(range) = range else { return };
		let mut files = self.state.files.write().await;
		let file = files.get_mut(path);
		if let Some(File::Opened(file)) = file {
			// For a given line and character offset, we need to compute a character offset in the file.
			let start: usize = file
				.text
				.lines()
				.take(range.start.line.to_usize())
				.map(|s| s.chars().count())
				.sum::<usize>() + range.start.line.to_usize()
				+ range.start.character.to_usize();
			let end: usize = file
				.text
				.lines()
				.take(range.end.line.to_usize())
				.map(|s| s.chars().count())
				.sum::<usize>() + range.end.line.to_usize()
				+ range.end.character.to_usize();
			file.text.replace_range(start..end, &text);
			file.version = version;
		}
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

	pub async fn find_rename_locations(
		&self,
		url: js::Url,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::FindRenameLocations(FindRenameLocationsRequest { url, position });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::FindRenameLocations(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let FindRenameLocationsResponse { locations } = response;

		Ok(locations)
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

		// Get the result the response.
		let GetDiagnosticsResponse { diagnostics } = response;

		Ok(diagnostics)
	}

	pub async fn get_references(
		&self,
		url: js::Url,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::GetReferences(GetReferencesRequest { url, position });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::GetReferences(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let GetReferencesResponse { locations } = response;

		Ok(locations)
	}

	pub async fn hover(&self, url: js::Url, position: Position) -> Result<Option<String>> {
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
		let text = response.text;

		Ok(text)
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
		let GotoDefinitionResponse { locations } = response;

		Ok(locations)
	}

	pub async fn completion(
		&self,
		url: js::Url,
		position: Position,
	) -> Result<Option<Vec<CompletionEntry>>> {
		// Create the request.
		let request = Request::Completion(CompletionRequest { url, position });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Completion(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let entries = response.entries;

		Ok(entries)
	}

	pub async fn transpile(&self, source: String) -> Result<TranspileOutput> {
		// Create the request.
		let request = Request::Transpile(TranspileRequest { source });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Transpile(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(TranspileOutput {
			transpiled_source: response.output_text,
			source_map: response.source_map_text,
		})
	}
}

impl Drop for Compiler {
	fn drop(&mut self) {
		if let Some(sender) = self.state.sender.lock().unwrap().take() {
			sender.send(None).ok();
		}
	}
}
