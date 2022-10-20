use crate::{builder::Builder, hash::Hash};
use anyhow::{anyhow, bail, Context, Result};
use deno_core::{serde_v8, v8};
use std::sync::Arc;

pub struct Compiler {
	_thread: Arc<std::thread::JoinHandle<()>>,
	sender: tokio::sync::mpsc::UnboundedSender<Option<Envelope>>,
}

struct Envelope {
	request: Request,
	sender: tokio::sync::oneshot::Sender<Result<Response>>,
}

#[derive(serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "camelCase")]
enum Request {
	Check(CheckRequest),
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", content = "content", rename_all = "camelCase")]
enum Response {
	Check(CheckResponse),
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct CheckRequest {
	package_hash: Hash,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckResponse {
	diagnostics: Vec<String>,
}

impl Compiler {
	#[must_use]
	pub fn new(builder: Builder) -> Compiler {
		// Create a channel to send requests to the compiler runtime.
		let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<Option<Envelope>>();

		// Spawn a thread for the compiler runtime to respond to requests.
		let thread = std::thread::spawn(move || {
			// Create a single threaded tokio runtime.
			let rt = tokio::runtime::Builder::new_current_thread()
				.enable_all()
				.build()
				.unwrap();
			rt.block_on(async move {
				let mut runtime = Runtime::new(builder);
				while let Some(envelope) = receiver.recv().await {
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
			});
		});

		Compiler {
			_thread: Arc::new(thread),
			sender,
		}
	}

	async fn request(&self, request: Request) -> Result<Response> {
		// Create a channel for the compiler runtime to send responses.
		let (sender, receiver) = tokio::sync::oneshot::channel();

		// Send the request.
		let envelope = Envelope { request, sender };
		self.sender
			.send(Some(envelope))
			.map_err(|_| anyhow!("Failed to send the request."))?;

		// Receive the response.
		let response = receiver
			.await
			.context("Failed to receive a response for the request.")?
			.context("The handler errored.")?;

		Ok(response)
	}

	pub async fn check(&self, package_hash: Hash) -> Result<Vec<String>> {
		// Create the request.
		let request = Request::Check(CheckRequest { package_hash });

		// Send the request and receive the response.
		let response = self.request(request).await?;
		let response = match response {
			Response::Check(response) => response,
			_ => bail!("Unexpected response type."),
		};

		// Get the result from the response.
		let diagnostics = response.diagnostics;

		Ok(diagnostics)
	}
}

impl Drop for Compiler {
	fn drop(&mut self) {
		self.sender.send(None).ok();
	}
}

struct Runtime {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
	runtime: deno_core::JsRuntime,
}

struct OpState {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js_compiler_snapshot"));

impl Runtime {
	/// Create a new Compiler (`typescript` inside a `deno_core::JsRuntime`).
	#[must_use]
	pub fn new(builder: Builder) -> Runtime {
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Build the tangram extension.
		let tangram_extension = deno_core::Extension::builder()
			.ops(vec![])
			.state({
				let builder = builder.clone();
				let main_runtime_handle = main_runtime_handle.clone();
				move |state| {
					state.put(Arc::new(OpState {
						builder: builder.clone(),
						main_runtime_handle: main_runtime_handle.clone(),
					}));
					Ok(())
				}
			})
			.build();

		// Create the js runtime.
		let runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
			extensions: vec![tangram_extension],
			module_loader: None,
			startup_snapshot: Some(deno_core::Snapshot::Static(SNAPSHOT)),
			..Default::default()
		});

		Runtime {
			builder,
			main_runtime_handle,
			runtime,
		}
	}

	pub fn handle(&mut self, request: Request) -> Result<Response> {
		// Create a scope to call the handle function.
		let mut scope = self.runtime.handle_scope();
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		// Get the handle function.
		let handle: v8::Local<v8::Function> =
			deno_core::JsRuntime::grab_global(&mut try_catch_scope, "handle")
				.context("Failed to get the handle function from the global scope.")?;

		// Call the handle function.
		let receiver = v8::undefined(&mut try_catch_scope).into();
		let request = serde_v8::to_v8(&mut try_catch_scope, request)
			.context("Failed to serialize the request.")?;
		let response = handle.call(&mut try_catch_scope, receiver, &[request]);

		// Handle an exception from js.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
			return Err(error.into());
		}
		let response = response.unwrap();

		// Deserialize the response.
		let response = serde_v8::from_v8(&mut try_catch_scope, response)
			.context("Failed to deserialize the response.")?;

		Ok(response)
	}
}
