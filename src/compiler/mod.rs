use self::{
	check::{CheckRequest, CheckResponse},
	completion::{CompletionRequest, CompletionResponse},
	definition::{DefinitionResponse, DefintionRequest},
	diagnostics::{DiagnosticsRequest, DiagnosticsResponse},
	format::{FormatRequest, FormatResponse},
	hover::{HoverRequest, HoverResponse},
	references::{ReferencesRequest, ReferencesResponse},
	rename::{RenameRequest, RenameResponse},
	syscall::syscall,
	transpile::{TranspileRequest, TranspileResponse},
};
pub use self::{
	files::{OpenedTrackedFile, TrackedFile, UnopenedTrackedFile},
	module_identifier::ModuleIdentifier,
	module_specifier::ModuleSpecifier,
	types::*,
};
use crate::Cli;
use anyhow::{anyhow, Context, Result};

mod analyze;
mod check;
mod completion;
mod definition;
mod diagnostics;
mod exception;
mod files;
mod format;
mod hover;
mod load;
mod metadata;
mod module_identifier;
mod module_specifier;
mod position;
mod range;
mod references;
mod rename;
mod resolve;
mod syscall;
mod transpile;
mod types;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", content = "request", rename_all = "snake_case")]
pub enum Request {
	Check(CheckRequest),
	Completion(CompletionRequest),
	Definition(DefintionRequest),
	Diagnostics(DiagnosticsRequest),
	Format(FormatRequest),
	Hover(HoverRequest),
	References(ReferencesRequest),
	Rename(RenameRequest),
	Transpile(TranspileRequest),
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", content = "response", rename_all = "snake_case")]
pub enum Response {
	Check(CheckResponse),
	Completion(CompletionResponse),
	Definition(DefinitionResponse),
	Diagnostics(DiagnosticsResponse),
	Format(FormatResponse),
	Hover(HoverResponse),
	References(ReferencesResponse),
	Rename(RenameResponse),
	Transpile(TranspileResponse),
}

pub type RequestSender = tokio::sync::mpsc::UnboundedSender<Option<(Request, ResponseSender)>>;
pub type RequestReceiver = tokio::sync::mpsc::UnboundedReceiver<Option<(Request, ResponseSender)>>;
pub type ResponseSender = tokio::sync::oneshot::Sender<Result<Response>>;
pub type _ResponseReceiver = tokio::sync::oneshot::Receiver<Result<Response>>;

impl Cli {
	async fn request(&self, request: Request) -> Result<Response> {
		// Create the request handler if necessary.
		let request_sender = self
			.inner
			.compiler_request_sender
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				// Create the request sender and receiver.
				let (request_sender, request_receiver) =
					tokio::sync::mpsc::unbounded_channel::<Option<(Request, ResponseSender)>>();

				// Spawn a thread for the request handler.
				std::thread::spawn({
					let cli = self.clone();
					move || handle_requests(cli, request_receiver)
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

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/compiler.heapsnapshot"));

fn handle_requests(cli: Cli, mut request_receiver: RequestReceiver) {
	// Create the isolate.
	let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);
	let mut isolate = v8::Isolate::new(params);

	// Create the context.
	let mut handle_scope = v8::HandleScope::new(&mut isolate);
	let context = v8::Context::new(&mut handle_scope);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Set the cli on the context.
	context.set_slot(&mut context_scope, cli);

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

	// Get the compiler.
	let compiler_string = v8::String::new(&mut context_scope, "compiler").unwrap();
	let compiler_object: v8::Local<v8::Object> = context
		.global(&mut context_scope)
		.get(&mut context_scope, compiler_string.into())
		.unwrap()
		.try_into()
		.unwrap();
	let default_string = v8::String::new(&mut context_scope, "default").unwrap();
	let compiler: v8::Local<v8::Function> = compiler_object
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

		// Call the compiler.
		let receiver = v8::undefined(&mut try_catch_scope).into();
		let response = compiler.call(&mut try_catch_scope, receiver, &[request]);

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
