pub use self::{module_identifier::ModuleIdentifier, types::*};
use self::{
	request::{Request, Response},
	syscall::syscall,
};
use crate::Cli;
use anyhow::{anyhow, Context, Result};
use std::{
	collections::HashMap,
	path::PathBuf,
	rc::Rc,
	sync::{Arc, Mutex},
	time::SystemTime,
};
use tokio::sync::RwLock;

mod check;
mod completion;
mod definition;
mod diagnostics;
mod exception;
mod files;
mod format;
mod hover;
mod load;
mod module_identifier;
mod references;
mod rename;
mod request;
mod resolve;
mod syscall;
mod transpile;
mod types;

#[derive(Clone)]
pub struct Compiler {
	cli: Cli,
	request_sender: Arc<Mutex<Option<RequestSender>>>,
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

type RequestSender = tokio::sync::mpsc::UnboundedSender<Option<(Request, ResponseSender)>>;
type RequestReceiver = tokio::sync::mpsc::UnboundedReceiver<Option<(Request, ResponseSender)>>;
type ResponseSender = tokio::sync::oneshot::Sender<Result<Response>>;
type _ResponseReceiver = tokio::sync::oneshot::Receiver<Result<Response>>;

impl Compiler {
	#[must_use]
	pub fn new(cli: Cli) -> Compiler {
		let state = State {
			files: RwLock::new(HashMap::default()),
		};
		Compiler {
			cli,
			request_sender: Arc::new(std::sync::Mutex::new(None)),
			state: Arc::new(state),
		}
	}

	async fn request(&self, request: Request) -> Result<Response> {
		// Create the request handler if necessary.
		let request_sender = self
			.request_sender
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				// Create the request sender and receiver.
				let (request_sender, request_receiver) =
					tokio::sync::mpsc::unbounded_channel::<Option<(Request, ResponseSender)>>();

				// Spawn a thread for the request handler.
				std::thread::spawn({
					let compiler = self.clone();
					let main_runtime_handle = tokio::runtime::Handle::current();
					move || handle_requests(compiler, main_runtime_handle, request_receiver)
				});

				request_sender
			})
			.clone();

		// Create a oneshot channel for the response.
		let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

		// Send the request.
		request_sender
			.send(Some((request, response_sender)))
			.map_err(|_| anyhow!("Failed to send the request."))?;

		// Receive the response.
		let response = response_receiver
			.await
			.context("Failed to receive a response for the request.")?
			.context("The handler errored.")?;

		Ok(response)
	}
}

impl Drop for Compiler {
	fn drop(&mut self) {
		// Attempt to shut down the request handler.
		if let Some(sender) = self.request_sender.lock().unwrap().take() {
			sender.send(None).ok();
		}
	}
}

struct ContextState {
	compiler: Compiler,
	main_runtime_handle: tokio::runtime::Handle,
}

fn handle_requests(
	compiler: Compiler,
	main_runtime_handle: tokio::runtime::Handle,
	mut request_receiver: RequestReceiver,
) {
	// Create the isolate.
	let params = v8::CreateParams::default();
	let mut isolate = v8::Isolate::new(params);

	// Create the context.
	let mut handle_scope = v8::HandleScope::new(&mut isolate);
	let context = v8::Context::new(&mut handle_scope);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Create the context state.
	let context_state = Rc::new(ContextState {
		compiler,
		main_runtime_handle,
	});

	// Set the context state on the context.
	context.set_slot(&mut context_scope, context_state);

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

	// Run the main script.
	let source = v8::String::new(&mut context_scope, include_str!("./main.js")).unwrap();
	let script = v8::Script::compile(&mut context_scope, source, None).unwrap();
	script.run(&mut context_scope).unwrap();

	// Get the handle function.
	let main_string = v8::String::new(&mut context_scope, "main").unwrap();
	let main: v8::Local<v8::Object> = context
		.global(&mut context_scope)
		.get(&mut context_scope, main_string.into())
		.unwrap()
		.try_into()
		.unwrap();
	let default_string = v8::String::new(&mut context_scope, "default").unwrap();
	let handle: v8::Local<v8::Function> = main
		.get(&mut context_scope, default_string.into())
		.unwrap()
		.try_into()
		.unwrap();

	while let Some(Some((request, response_sender))) = request_receiver.blocking_recv() {
		// Create a try catch scope.
		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);

		// Serialize the request.
		let request = match serde_v8::to_v8(&mut try_catch_scope, request)
			.context("Failed to serialize the request.")
		{
			Ok(request) => request,
			Err(error) => {
				response_sender.send(Err(error)).unwrap();
				continue;
			},
		};

		// Call the handle function.
		let receiver = v8::undefined(&mut try_catch_scope).into();
		let response = handle.call(&mut try_catch_scope, receiver, &[request]);

		// Handle a js exception.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let exception = self::exception::render(&mut scope, exception);
			response_sender.send(Err(anyhow!("{exception}"))).unwrap();
			continue;
		}

		// Deserialize the response.
		let response = response.unwrap();
		let response = match serde_v8::from_v8(&mut try_catch_scope, response)
			.context("Failed to deserialize the response.")
		{
			Ok(response) => response,
			Err(error) => {
				response_sender.send(Err(error)).unwrap();
				continue;
			},
		};

		// Send the response.
		response_sender.send(Ok(response)).unwrap();
	}
}
