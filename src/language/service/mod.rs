use self::syscall::syscall;
use crate::{
	error::{Error, Result, WrapErr},
	instance::Instance,
};
use std::sync::{Arc, Weak};

pub mod check;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod doc;
pub mod error;
mod exception;
pub mod format;
pub mod hover;
pub mod metadata;
pub mod references;
pub mod rename;
pub mod symbols;
mod syscall;

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum Request {
	Check(check::Request),
	Completion(completion::Request),
	Definition(definition::Request),
	Diagnostics(diagnostics::Request),
	Doc(doc::Request),
	Format(format::Request),
	Hover(hover::Request),
	Metadata(metadata::Request),
	References(references::Request),
	Rename(rename::Request),
	Symbols(symbols::Request),
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "kind", content = "response", rename_all = "snake_case")]
pub enum Response {
	Check(check::Response),
	Completion(completion::Response),
	Definition(definition::Response),
	Diagnostics(diagnostics::Response),
	Doc(doc::Response),
	Format(format::Response),
	Hover(hover::Response),
	Metadata(metadata::Response),
	References(references::Response),
	Rename(rename::Response),
	Symbols(symbols::Response),
}

pub type RequestSender = tokio::sync::mpsc::UnboundedSender<(Request, ResponseSender)>;
pub type RequestReceiver = tokio::sync::mpsc::UnboundedReceiver<(Request, ResponseSender)>;
pub type ResponseSender = tokio::sync::oneshot::Sender<Result<Response>>;
pub type _ResponseReceiver = tokio::sync::oneshot::Receiver<Result<Response>>;

impl Instance {
	pub async fn handle_language_service_request(
		self: &Arc<Self>,
		request: Request,
	) -> Result<Response> {
		// Spawn the language service if necessary.
		let request_sender = self
			.language_service_request_sender
			.lock()
			.unwrap()
			.get_or_insert_with(|| {
				// Create the language service request sender and receiver.
				let (request_sender, request_receiver) =
					tokio::sync::mpsc::unbounded_channel::<(Request, ResponseSender)>();

				// Spawn a thread to run the language service.
				std::thread::spawn({
					let tg = Arc::downgrade(self);
					move || run_language_service(tg, request_receiver)
				});

				request_sender
			})
			.clone();

		// Create a oneshot channel for the response.
		let (response_sender, response_receiver) = tokio::sync::oneshot::channel();

		// Send the request.
		request_sender
			.send((request, response_sender))
			.ok()
			.wrap_err("Failed to send the language service request.")?;

		// Receive the response.
		let response = response_receiver
			.await
			.ok()
			.wrap_err("Failed to receive a response for the language service request.")?
			.wrap_err("The language service returned an error.")?;

		Ok(response)
	}
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/language_service.heapsnapshot"));

/// Run the language service.
fn run_language_service(tg: Weak<Instance>, mut request_receiver: RequestReceiver) {
	// Create the isolate.
	let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);
	let mut isolate = v8::Isolate::new(params);

	// Create the context.
	let mut handle_scope = v8::HandleScope::new(&mut isolate);
	let context = v8::Context::new(&mut handle_scope);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Set the instance on the context.
	context.set_slot(&mut context_scope, tg);

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

	// Get the handle function.
	let handle_string = v8::String::new(&mut context_scope, "handle").unwrap();
	let handle_function: v8::Local<v8::Function> = context
		.global(&mut context_scope)
		.get(&mut context_scope, handle_string.into())
		.unwrap()
		.try_into()
		.unwrap();

	while let Some((request, response_sender)) = request_receiver.blocking_recv() {
		// Create a try catch scope.
		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);

		// Serialize the request.
		let request = match serde_v8::to_v8(&mut try_catch_scope, request)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the request.")
		{
			Ok(request) => request,
			Err(error) => {
				response_sender.send(Err(error)).unwrap();
				continue;
			},
		};

		// Call the handle function.
		let receiver = v8::undefined(&mut try_catch_scope).into();
		let response = handle_function.call(&mut try_catch_scope, receiver, &[request]);

		// Handle a js exception.
		let Some(response) = response else {
			let exception = try_catch_scope.exception().unwrap();
			let error =
				self::Error::from_language_service_exception(&mut try_catch_scope, exception);
			response_sender.send(Err(error)).unwrap();
			continue;
		};

		// Deserialize the response.
		let response = match serde_v8::from_v8(&mut try_catch_scope, response)
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the response.")
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
