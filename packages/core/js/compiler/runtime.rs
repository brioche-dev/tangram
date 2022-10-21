use crate::builder::Builder;
use crate::hash::Hash;
use anyhow::{Context, Result};
use deno_core::{serde_v8, v8};
use std::sync::Arc;

pub struct Runtime {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
	runtime: deno_core::JsRuntime,
}

struct OpState {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js_compiler_snapshot"));

#[derive(serde::Serialize)]
#[serde(tag = "type", content = "content", rename_all = "camelCase")]
pub enum Request {
	Check(CheckRequest),
}

#[derive(serde::Deserialize)]
#[serde(tag = "type", content = "content", rename_all = "camelCase")]
pub enum Response {
	Check(CheckResponse),
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
	pub package_hash: Hash,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResponse {
	pub diagnostics: Vec<String>,
}

impl Runtime {
	/// Create a new Compiler (`typescript` inside a `deno_core::JsRuntime`).
	#[must_use]
	pub fn new(builder: Builder) -> Runtime {
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Build the tangram extension.
		let tangram_extension = deno_core::Extension::builder()
			.ops(vec![op_example::decl()])
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

#[deno_core::op]
fn op_example() -> String {
	futures::executor::block_on(async move { "It worked!".to_owned() })
}
