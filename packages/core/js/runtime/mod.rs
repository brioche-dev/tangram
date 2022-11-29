use self::module_loader::ModuleLoader;
use crate::{
	builder::Builder,
	expression::{Expression, Target},
	hash::Hash,
	js::{self, compiler::Compiler},
};
use anyhow::{anyhow, bail, Context, Result};
use deno_core::serde_v8;
use std::{
	cell::RefCell,
	future::Future,
	rc::Rc,
	sync::{Arc, Mutex},
};
use tokio::io::AsyncReadExt;

mod cdp;
mod module_loader;

pub struct Runtime {
	runtime: deno_core::JsRuntime,
	state: Arc<State>,
	inspector_session: deno_core::LocalInspectorSession,
	context_id: u64,
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js_runtime_snapshot"));

#[derive(Clone)]
struct State {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
	name: Arc<Mutex<Option<String>>>,
	args: Arc<Mutex<Option<Hash>>>,
	output: Arc<Mutex<Option<Hash>>>,
}

impl Runtime {
	pub async fn new(
		builder: Builder,
		main_runtime_handle: tokio::runtime::Handle,
	) -> Result<Runtime> {
		// Create the compiler.
		let compiler = Compiler::new(builder.clone());

		// Create the state.
		let state = Arc::new(State {
			builder,
			main_runtime_handle: main_runtime_handle.clone(),
			name: Arc::new(Mutex::new(None)),
			args: Arc::new(Mutex::new(None)),
			output: Arc::new(Mutex::new(None)),
		});

		// Create the module loader.
		let module_loader = Rc::new(ModuleLoader::new(
			compiler.clone(),
			main_runtime_handle.clone(),
		));

		// Build the tangram extension.
		let tangram_extension = deno_core::Extension::builder()
			.ops(vec![
				op_tg_get_hash::decl(),
				op_tg_get_name::decl(),
				op_tg_get_args::decl(),
				op_tg_return::decl(),
				op_tg_print::decl(),
				op_tg_serialize::decl(),
				op_tg_deserialize::decl(),
				op_tg_add_blob::decl(),
				op_tg_get_blob::decl(),
				op_tg_add_expression::decl(),
				op_tg_get_expression::decl(),
				op_tg_evaluate::decl(),
			])
			.state({
				let state = Arc::clone(&state);
				move |op_state| {
					let state = Arc::clone(&state);
					op_state.put(state);
					Ok(())
				}
			})
			.build();

		// Create the js runtime.
		let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
			source_map_getter: Some(
				Box::new(Rc::clone(&module_loader)) as Box<dyn deno_core::SourceMapGetter>
			),
			module_loader: Some(Rc::clone(&module_loader) as Rc<dyn deno_core::ModuleLoader>),
			extensions: vec![tangram_extension],
			startup_snapshot: Some(deno_core::Snapshot::Static(SNAPSHOT)),
			..Default::default()
		});

		// Create the v8 inspector session.
		let mut inspector_session = runtime.inspector().borrow().create_local_session();

		// Enable the inspector runtime.
		futures::try_join!(
			inspector_session.post_message::<()>("Runtime.enable", None),
			runtime.run_event_loop(false),
		)?;

		// Retrieve the inspector session context id.
		let mut context_id: u64 = 0;
		for notification in inspector_session.notifications() {
			let method = notification.get("method").unwrap().as_str().unwrap();
			let params = notification.get("params").unwrap();
			if method == "Runtime.executionContextCreated" {
				context_id = params
					.get("context")
					.unwrap()
					.get("id")
					.unwrap()
					.as_u64()
					.unwrap();
			}
		}

		let runtime = Runtime {
			runtime,
			state,
			inspector_session,
			context_id,
		};
		Ok(runtime)
	}

	#[allow(clippy::too_many_lines)]
	pub async fn run(&mut self, hash: Hash, target: &Target) -> Result<Hash> {
		// Lock the builder.
		let builder = self.state.builder.lock_shared().await?;

		// Get the package hash.
		let package_hash = builder.evaluate(target.package, hash).await?;

		// Set the name and args in the state.
		self.state.name.lock().unwrap().replace(target.name.clone());
		self.state.args.lock().unwrap().replace(target.args);

		// Get the package's entrypoint.
		let entrypoint = builder
			.get_package_entrypoint(package_hash)
			.context("Failed to retrieve the package entrypoint.")?
			.context("The package must have an entrypoint.")?;

		// Create the URL.
		let url = js::Url::new_hash_target(package_hash, entrypoint);

		// Instantiate and evaluate the module.
		let module_id = self
			.runtime
			.load_side_module(&url.clone().into(), None)
			.await?;
		let evaluate_receiver = self.runtime.mod_evaluate(module_id);
		self.runtime.run_event_loop(false).await?;
		evaluate_receiver.await.unwrap()?;

		// Retrieve the output.
		let output_hash = self
			.state
			.output
			.lock()
			.unwrap()
			.take()
			.context("The process did not return a value.")?;

		Ok(output_hash)
	}

	pub async fn repl(&mut self, code: &str) -> Result<Option<String>, String> {
		// If the code begins with an open curly and does not end in a semicolon, wrap it in parens to make it an ExpressionStatement instead of a BlockStatement.
		let code = if code.trim_start().starts_with('{') && !code.trim_end().ends_with(';') {
			format!("({code})")
		} else {
			code.to_owned()
		};

		// Evaluate the code.
		let evaluate_response: cdp::EvaluateResponse = match futures::try_join!(
			self.inspector_session.post_message(
				"Runtime.evaluate",
				Some(cdp::EvaluateArgs {
					context_id: Some(self.context_id),
					repl_mode: Some(true),
					expression: code,
					object_group: None,
					include_command_line_api: None,
					silent: None,
					return_by_value: None,
					generate_preview: Some(true),
					user_gesture: None,
					await_promise: None,
					throw_on_side_effect: None,
					timeout: None,
					disable_breaks: None,
					allow_unsafe_eval_blocked_by_csp: None,
					unique_context_id: None,
				}),
			),
			self.runtime.run_event_loop(false),
		) {
			Ok((response, _)) => serde_json::from_value(response).unwrap(),
			Err(error) => {
				return Err(error.to_string());
			},
		};

		// If there was an error, return its description.
		if let Some(exception_details) = evaluate_response.exception_details {
			return Err(exception_details.exception.unwrap().description.unwrap());
		}

		// If the evaluation produced a value, return it.
		if let Some(value) = evaluate_response.result.value {
			let output = serde_json::to_string_pretty(&value).unwrap();
			return Ok(Some(output));
		}

		// Otherwise, stringify the evaluation response's result.
		let function = r#"
			function stringifyFunction(value) {
				return stringify(value);
			}
		"#;
		let call_function_on_response: cdp::CallFunctionOnResponse = match futures::try_join!(
			self.inspector_session.post_message(
				"Runtime.callFunctionOn",
				Some(cdp::CallFunctionOnArgs {
					function_declaration: function.to_string(),
					object_id: None,
					arguments: Some(vec![(&evaluate_response.result).into()]),
					silent: None,
					return_by_value: None,
					generate_preview: None,
					user_gesture: None,
					await_promise: None,
					execution_context_id: Some(self.context_id),
					object_group: None,
					throw_on_side_effect: None
				}),
			),
			self.runtime.run_event_loop(false),
		) {
			Ok((response, _)) => serde_json::from_value(response).unwrap(),
			Err(error) => return Err(error.to_string()),
		};

		// If there was an error, return its description.
		if let Some(exception_details) = call_function_on_response.exception_details {
			return Err(exception_details.exception.unwrap().description.unwrap());
		}

		// Retrieve the output.
		let Some(output) = call_function_on_response.result.value else {
			return Err("An unexpected error occurred.".to_owned());
		};

		// Get the output as a string.
		let output = output.as_str().unwrap().to_owned();

		Ok(Some(output))
	}
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_get_hash(
	_state: Rc<RefCell<deno_core::OpState>>,
	url: js::Url,
) -> Result<Hash, deno_core::error::AnyError> {
	let package_hash = match url {
		js::Url::HashModule(js::compiler::url::HashModule { package_hash, .. }) => package_hash,
		_ => bail!("Invalid URL."),
	};
	Ok(package_hash)
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_get_name(
	state: Rc<RefCell<deno_core::OpState>>,
) -> Result<String, deno_core::error::AnyError> {
	op_sync(state, |state| {
		let name = state.name.lock().unwrap();
		let name = name.as_ref().cloned().unwrap();
		Ok(name)
	})
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_get_args(
	state: Rc<RefCell<deno_core::OpState>>,
) -> Result<Hash, deno_core::error::AnyError> {
	op_sync(state, |state| {
		let args = state.args.lock().unwrap();
		let args = args.as_ref().copied().unwrap();
		Ok(args)
	})
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_return(
	state: Rc<RefCell<deno_core::OpState>>,
	value: Hash,
) -> Result<(), deno_core::error::AnyError> {
	op_sync(state, |state| {
		let mut output = state.output.lock().unwrap();
		output.replace(value);
		Ok(())
	})
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_print(string: String) -> Result<(), deno_core::error::AnyError> {
	println!("{string}");
	Ok(())
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
enum SerializationFormat {
	#[serde(rename = "toml")]
	Toml,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_serialize(
	format: SerializationFormat,
	value: serde_json::Value,
) -> Result<String, deno_core::error::AnyError> {
	match format {
		SerializationFormat::Toml => {
			let value = toml::to_string(&value)?;
			Ok(value)
		},
	}
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tg_deserialize(
	format: SerializationFormat,
	string: String,
) -> Result<serde_json::Value, deno_core::error::AnyError> {
	match format {
		SerializationFormat::Toml => {
			let value = toml::from_str(&string)?;
			Ok(value)
		},
	}
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tg_add_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	blob: serde_v8::ZeroCopyBuf,
) -> Result<Hash, deno_core::error::AnyError> {
	op_async(state, |state| async move {
		let hash = state
			.builder
			.lock_shared()
			.await?
			.add_blob(blob.as_ref())
			.await?;
		Ok::<_, anyhow::Error>(hash)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tg_get_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<serde_v8::ZeroCopyBuf, deno_core::error::AnyError> {
	op_async(state, |state| async move {
		let mut blob = state.builder.lock_shared().await?.get_blob(hash).await?;
		let mut bytes = Vec::new();
		blob.read_to_end(&mut bytes).await?;
		let output = serde_v8::ZeroCopyBuf::ToV8(Some(bytes.into_boxed_slice()));
		Ok::<_, anyhow::Error>(output)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tg_add_expression(
	state: Rc<RefCell<deno_core::OpState>>,
	expression: Expression,
) -> Result<Hash, deno_core::error::AnyError> {
	op_async(state, |state| async move {
		let hash = state
			.builder
			.lock_shared()
			.await?
			.add_expression(&expression)
			.await?;
		Ok::<_, anyhow::Error>(hash)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tg_get_expression(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Option<Expression>, deno_core::error::AnyError> {
	op_async(state, |state| async move {
		let expression = state
			.builder
			.lock_shared()
			.await?
			.try_get_expression_local(hash)?;
		Ok::<_, anyhow::Error>(expression)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tg_evaluate(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Hash, deno_core::error::AnyError> {
	op_async(state, |state| async move {
		let output = state
			.builder
			.lock_shared()
			.await?
			.evaluate(hash, hash)
			.await
			.map_err(|e| anyhow!("{e:#}"))?;
		Ok::<_, anyhow::Error>(output)
	})
	.await
}

#[allow(clippy::needless_pass_by_value)]
fn op_sync<R, F>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<R, deno_core::error::AnyError>
where
	F: FnOnce(Arc<State>) -> Result<R, deno_core::error::AnyError>,
{
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<State>>();
		Arc::clone(state)
	};
	let output = f(state)?;
	Ok(output)
}

#[allow(clippy::needless_pass_by_value)]
async fn op_async<R, F, Fut>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<R, deno_core::error::AnyError>
where
	R: 'static + Send,
	F: FnOnce(Arc<State>) -> Fut,
	Fut: 'static + Send + Future<Output = Result<R, deno_core::error::AnyError>>,
{
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<State>>();
		Arc::clone(state)
	};
	let main_runtime_handle = state.main_runtime_handle.clone();
	let output = main_runtime_handle.spawn(f(state)).await.unwrap()?;
	Ok(output)
}
