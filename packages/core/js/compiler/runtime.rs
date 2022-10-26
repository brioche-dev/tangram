use super::{Compiler, Diagnostic, File};
use crate::js;
use anyhow::{Context, Result};
use deno_core::{serde_v8, v8};
use futures::{future::try_join_all, Future};
use std::{cell::RefCell, collections::BTreeMap, env, rc::Rc, sync::Arc};
use tokio::sync::oneshot;

// TODO: Compress this snapshot with zstd to save 20MB of binary size (and presumably some startup time too).
const TS_SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js_compiler_snapshot"));

pub struct Runtime {
	runtime: deno_core::JsRuntime,
	_state: Arc<OpState>,
}

struct OpState {
	compiler: Compiler,
}

impl Runtime {
	#[must_use]
	pub fn new(compiler: Compiler) -> Runtime {
		let state = Arc::new(OpState { compiler });

		// Build the tangram extension.
		let tangram_extension = deno_core::Extension::builder()
			.ops(vec![
				op_tg_load::decl(),
				op_tg_opened_files::decl(),
				op_tg_print::decl(),
				op_tg_resolve::decl(),
				op_tg_version::decl(),
			])
			.state({
				{
					let state: Arc<OpState> = Arc::clone(&state);
					move |state_map| {
						state_map.put(Arc::clone(&state));
						Ok(())
					}
				}
			})
			.build();

		// Create the js runtime.
		let runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
			extensions: vec![tangram_extension],
			module_loader: None,
			startup_snapshot: Some(deno_core::Snapshot::Static(TS_SNAPSHOT)),
			..Default::default()
		});

		Runtime {
			runtime,
			_state: state,
		}
	}

	pub async fn handle(&mut self, request: Request) -> Result<Response> {
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
		let output = handle.call(&mut try_catch_scope, receiver, &[request]);

		// Handle an exception from js.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
			return Err(error.into());
		}

		// If there was no caught exception then retrieve the return value.
		let output = output.unwrap();

		// Move the return value to the global scope.
		let output = v8::Global::new(&mut try_catch_scope, output);
		drop(try_catch_scope);
		drop(scope);

		// Resolve the value.
		let output = self.runtime.resolve_value(output).await?;

		// Deserialize the response.
		let mut scope = self.runtime.handle_scope();
		let output = v8::Local::new(&mut scope, output);
		let response =
			serde_v8::from_v8(&mut scope, output).context("Failed to deserialize the response.")?;
		drop(scope);

		Ok(response)
	}
}

#[derive(Debug)]
pub struct Envelope {
	pub request: Request,
	pub sender: oneshot::Sender<Result<Response>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "type", content = "request", rename_all = "snake_case")]
pub enum Request {
	Check(CheckRequest),
	GetDiagnostics(GetDiagnosticsRequest),
	GetDefinition(GetDefinitionRequest),
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "type", content = "response", rename_all = "snake_case")]
pub enum Response {
	Check(CheckResponse),
	GetDiagnostics(GetDiagnosticsResponse),
	GetDefinition(GetDefinitionResponse),
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRequest {
	pub urls: Vec<js::Url>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResponse {
	pub diagnostics: BTreeMap<js::Url, Vec<Diagnostic>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDiagnosticsRequest {}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDiagnosticsResponse {
	pub diagnostics: BTreeMap<js::Url, Vec<Diagnostic>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDefinitionRequest {}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDefinitionResponse {}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadOutput {
	text: String,
	version: i32,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_load(
	state: Rc<RefCell<deno_core::OpState>>,
	url: js::Url,
) -> Result<LoadOutput, deno_core::error::AnyError> {
	op_sync(state, |state| async move {
		let text = state.compiler.load(&url).await?;
		let version = state.compiler.get_version(&url).await?;
		Ok(LoadOutput { text, version })
	})
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_opened_files(
	state: Rc<RefCell<deno_core::OpState>>,
) -> Result<Vec<js::Url>, deno_core::error::AnyError> {
	op_sync(state, |state| async move {
		let files = state.compiler.state.files.read().await;
		let urls = try_join_all(
			files
				.iter()
				.filter_map(|(path, file)| match file {
					File::Opened(_) => Some(path),
					File::Unopened(_) => None,
				})
				.map(|path| js::Url::new_for_module_path(path)),
		)
		.await?;
		Ok(urls)
	})
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_print(string: String) -> Result<(), deno_core::error::AnyError> {
	eprintln!("{string}");
	Ok(())
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_resolve(
	state: Rc<RefCell<deno_core::OpState>>,
	specifier: String,
	referrer: Option<js::Url>,
) -> Result<js::Url, deno_core::error::AnyError> {
	op_sync(state, |state| async move {
		let url = state
			.compiler
			.resolve(&specifier, referrer.as_ref())
			.await?;
		Ok(url)
	})
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_version(
	state: Rc<RefCell<deno_core::OpState>>,
	url: js::Url,
) -> Result<String, deno_core::error::AnyError> {
	op_sync(state, |state| async move {
		let version = state.compiler.get_version(&url).await?;
		Ok(version.to_string())
	})
}

async fn op<R, F, Fut>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<R, deno_core::error::AnyError>
where
	R: 'static + Send,
	F: FnOnce(Arc<OpState>) -> Fut,
	Fut: 'static + Send + Future<Output = Result<R, deno_core::error::AnyError>>,
{
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = f(state).await?;
	Ok(output)
}

fn op_sync<R, F, Fut>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<R, deno_core::error::AnyError>
where
	R: 'static + Send,
	F: FnOnce(Arc<OpState>) -> Fut,
	Fut: 'static + Send + Future<Output = Result<R, deno_core::error::AnyError>>,
{
	futures::executor::block_on(op(state, f))
}
