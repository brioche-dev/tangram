pub use self::{module_identifier::ModuleIdentifier, types::*};
use self::{
	request::{
		CheckRequest, CompletionRequest, DefinitionResponse, DefintionRequest, DiagnosticsRequest,
		DiagnosticsResponse, HoverRequest, ReferencesRequest, ReferencesResponse,
		RenameLocationsResponse, RenameRequest, Request, Response, TranspileRequest,
	},
	syscall::syscall,
};
use crate::Cli;
use anyhow::{anyhow, bail, Context, Result};
use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	rc::Rc,
	sync::{Arc, Mutex},
	time::SystemTime,
};
use tokio::sync::RwLock;

mod exception;
mod load;
mod module_identifier;
mod request;
mod resolve;
mod syscall;
mod types;

#[derive(Clone)]
pub struct Compiler {
	cli: Cli,
	sender: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<Option<Envelope>>>>>,
	state: Arc<State>,
}

pub struct State {
	files: RwLock<HashMap<PathBuf, File, fnv::FnvBuildHasher>>,
}

#[derive(Debug)]
enum File {
	Opened(OpenedFile),
	Unopened(UnopenedFile),
}

#[derive(Debug)]
struct OpenedFile {
	module_identifier: ModuleIdentifier,
	version: i32,
	text: String,
}

#[derive(Debug)]
struct UnopenedFile {
	_module_identifier: ModuleIdentifier,
	version: i32,
	modified: SystemTime,
}

pub struct Envelope {
	pub request: Request,
	pub sender: tokio::sync::oneshot::Sender<Result<Response>>,
}

impl Compiler {
	#[must_use]
	pub fn new(cli: Cli) -> Compiler {
		let state = State {
			files: RwLock::new(HashMap::default()),
		};
		Compiler {
			cli,
			sender: Arc::new(std::sync::Mutex::new(None)),
			state: Arc::new(state),
		}
	}

	fn runtime_sender(&self) -> tokio::sync::mpsc::UnboundedSender<Option<Envelope>> {
		let main_runtime_handle = tokio::runtime::Handle::current();
		let mut lock = self.sender.lock().unwrap();
		if let Some(sender) = lock.as_ref() {
			sender.clone()
		} else {
			// Create a channel to send requests to the compiler runtime.
			let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Option<Envelope>>();

			// Spawn a thread for the compiler runtime to respond to requests.
			std::thread::spawn({
				let compiler = self.clone();
				move || {
					let mut runtime = Runtime::new(compiler, main_runtime_handle);
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

	async fn request(&self, request: Request) -> Result<Response> {
		// Create a oneshot channel for js to send the response.
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
	pub async fn get_version(&self, module_identifier: &ModuleIdentifier) -> Result<i32> {
		// Get the path for the module identifier, or return version 0 for modules whose contents never change.
		let path = match module_identifier {
			// Path modules change when the file at their path changes.
			ModuleIdentifier::Path {
				package_path,
				module_path,
			} => package_path.join(module_path),

			// Library, core, and hash modules never change, so we can always return 0.
			ModuleIdentifier::Lib { .. }
			| ModuleIdentifier::Core { .. }
			| ModuleIdentifier::Hash { .. } => {
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
						_module_identifier: module_identifier.clone(),
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
		let Ok(url) = ModuleIdentifier::for_path(path).await else { return };
		let file = File::Opened(OpenedFile {
			module_identifier: url,
			version,
			text,
		});
		self.state.files.write().await.insert(path.to_owned(), file);
	}

	pub async fn close_file(&self, path: &Path) {
		self.state.files.write().await.remove(path);
	}

	pub async fn change_file(&self, path: &Path, version: i32, range: Option<Range>, text: String) {
		// Lock the files.
		let mut files = self.state.files.write().await;

		// Get the file.
		let Some(File::Opened(file)) = files.get_mut(path) else { return };

		// Convert the range to bytes.
		let range = if let Some(range) = range {
			let start = byte_index_for_line_and_character_index(
				&file.text,
				range.start.line as usize,
				range.start.character as usize,
			);
			let end = byte_index_for_line_and_character_index(
				&file.text,
				range.end.line as usize,
				range.end.character as usize,
			);
			start..end
		} else {
			0..file.text.len()
		};

		// Replace the text and update the version.
		file.text.replace_range(range, &text);
		file.version = version;
	}
}

fn byte_index_for_line_and_character_index(string: &str, line: usize, character: usize) -> usize {
	let mut byte_index = 0;
	let mut line_index = 0;
	let mut character_index = 0;
	for code_point in string.chars() {
		if line_index == line && character_index == character {
			return byte_index;
		}
		byte_index += code_point.len_utf8();
		character_index += 1;
		if code_point == '\n' {
			line_index += 1;
			character_index = 0;
		}
	}
	byte_index
}

impl Compiler {
	/// Get all diagnostics for a package.
	pub async fn check(
		&self,
		module_identifiers: Vec<ModuleIdentifier>,
	) -> Result<BTreeMap<ModuleIdentifier, Vec<Diagnostic>>> {
		// Create the request.
		let request = Request::Check(CheckRequest { module_identifiers });

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

	pub async fn rename(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::Rename(RenameRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Rename(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let RenameLocationsResponse { locations } = response;

		Ok(locations)
	}

	pub async fn get_diagnostics(&self) -> Result<BTreeMap<ModuleIdentifier, Vec<Diagnostic>>> {
		// Create the request.
		let request = Request::Diagnostics(DiagnosticsRequest {});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Diagnostics(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result the response.
		let DiagnosticsResponse { diagnostics } = response;

		Ok(diagnostics)
	}

	pub async fn get_references(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::References(ReferencesRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::References(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let ReferencesResponse { locations } = response;

		Ok(locations)
	}

	pub async fn hover(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<String>> {
		// Create the request.
		let request = Request::Hover(HoverRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Hover(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let text = response.text;

		Ok(text)
	}

	pub async fn goto_definition(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<Vec<Location>>> {
		// Create the request.
		let request = Request::Definition(DefintionRequest {
			module_identifier,
			position,
		});

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Definition(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let DefinitionResponse { locations } = response;

		Ok(locations)
	}

	pub async fn completion(
		&self,
		module_identifier: ModuleIdentifier,
		position: Position,
	) -> Result<Option<Vec<CompletionEntry>>> {
		// Create the request.
		let request = Request::Completion(CompletionRequest {
			module_identifier,
			position,
		});

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

	pub async fn transpile(&self, text: String) -> Result<TranspileOutput> {
		// Create the request.
		let request = Request::Transpile(TranspileRequest { text });

		// Send the request and receive the response.
		let response = self.request(request).await?;

		// Get the response.
		let response = match response {
			Response::Transpile(response) => response,
			_ => bail!("Unexpected response type."),
		};

		Ok(TranspileOutput {
			transpiled: response.output_text,
			source_map: response.source_map_text,
		})
	}
}

impl Drop for Compiler {
	fn drop(&mut self) {
		if let Some(sender) = self.sender.lock().unwrap().take() {
			sender.send(None).ok();
		}
	}
}

pub struct Runtime {
	isolate: v8::OwnedIsolate,
	context: v8::Global<v8::Context>,
}

struct ContextState {
	compiler: Compiler,
	main_runtime_handle: tokio::runtime::Handle,
}

impl Runtime {
	#[must_use]
	pub fn new(compiler: Compiler, main_runtime_handle: tokio::runtime::Handle) -> Runtime {
		// Create the isolate.
		let params = v8::CreateParams::default();
		let mut isolate = v8::Isolate::new(params);
		isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 10);

		// Create the context.
		let mut handle_scope = v8::HandleScope::new(&mut isolate);
		let context = v8::Context::new(&mut handle_scope);
		let context = v8::Global::new(&mut handle_scope, context);
		drop(handle_scope);

		// Create the context state.
		let context_state = Rc::new(ContextState {
			compiler,
			main_runtime_handle,
		});

		// Enter the context.
		let mut handle_scope = v8::HandleScope::new(&mut isolate);
		let context = v8::Local::new(&mut handle_scope, &context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Set the context state on the context.
		context.set_slot(&mut context_scope, context_state);

		// Run the main script.
		let source = v8::String::new(&mut context_scope, include_str!("./main.js")).unwrap();
		let script = v8::Script::compile(&mut context_scope, source, None).unwrap();
		script.run(&mut context_scope).unwrap();

		// Add the syscall function to the global.
		let syscall_string = v8::String::new(&mut context_scope, "syscall").unwrap();
		let syscall_function = v8::Function::new(&mut context_scope, syscall).unwrap();
		context
			.global(&mut context_scope)
			.set(
				&mut context_scope,
				syscall_string.into(),
				syscall_function.into(),
			)
			.unwrap();

		// Exit the context.
		let context = v8::Global::new(&mut context_scope, context);
		drop(context_scope);
		drop(handle_scope);

		Runtime { isolate, context }
	}

	pub fn handle(&mut self, request: Request) -> Result<Response> {
		// Enter the context.
		let mut handle_scope = v8::HandleScope::new(&mut self.isolate);
		let context = v8::Local::new(&mut handle_scope, &self.context);
		let mut scope = v8::ContextScope::new(&mut handle_scope, context);

		// Create a scope to call the handle function.
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		// Get the handle function.
		let main_string = v8::String::new(&mut try_catch_scope, "main").unwrap();
		let main: v8::Local<v8::Object> = context
			.global(&mut try_catch_scope)
			.get(&mut try_catch_scope, main_string.into())
			.unwrap()
			.try_into()
			.unwrap();
		let default_string = v8::String::new(&mut try_catch_scope, "default").unwrap();
		let handle: v8::Local<v8::Function> = main
			.get(&mut try_catch_scope, default_string.into())
			.unwrap()
			.try_into()
			.unwrap();

		// Serialize the request.
		let request = serde_v8::to_v8(&mut try_catch_scope, request)
			.context("Failed to serialize the request.")?;

		// Call the handle function.
		let receiver = v8::undefined(&mut try_catch_scope).into();
		let output = handle.call(&mut try_catch_scope, receiver, &[request]);

		// Handle a js exception.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let exception = self::exception::render(&mut scope, exception);
			bail!("{exception}");
		}

		// Deserialize the response.
		let output = output.unwrap();
		let response = serde_v8::from_v8(&mut try_catch_scope, output)
			.context("Failed to deserialize the response.")?;

		Ok(response)
	}
}
