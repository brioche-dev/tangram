use self::module_loader::ModuleLoader;
use crate::{
	builder::Builder,
	expression::{Expression, Js},
	hash::Hash,
	js::{self, compiler::Compiler},
};
use anyhow::{bail, Context, Result};
use deno_core::{serde_v8, v8};
use std::{cell::RefCell, future::Future, rc::Rc, sync::Arc};
use tokio::io::AsyncReadExt;

mod cdp;
mod module_loader;

pub struct Runtime {
	builder: Builder,
	_compiler: Compiler,
	_main_runtime_handle: tokio::runtime::Handle,
	runtime: deno_core::JsRuntime,
	inspector_session: deno_core::LocalInspectorSession,
	context_id: u64,
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/js_runtime_snapshot"));

#[derive(Clone)]
struct OpState {
	builder: Builder,
	main_runtime_handle: tokio::runtime::Handle,
}

impl Runtime {
	pub async fn new(
		builder: Builder,
		main_runtime_handle: tokio::runtime::Handle,
	) -> Result<Runtime> {
		// Create the compiler.
		let compiler = Compiler::new(builder.clone());

		// Create the module loader.
		let module_loader = Rc::new(ModuleLoader::new(
			compiler.clone(),
			main_runtime_handle.clone(),
		));

		// Build the tangram extension.
		let tangram_extension = deno_core::Extension::builder()
			.ops(vec![
				op_tg_print::decl(),
				op_tg_deserialize::decl(),
				op_tg_add_blob::decl(),
				op_tg_get_blob::decl(),
				op_tg_add_expression::decl(),
				op_tg_get_expression::decl(),
				op_tg_evaluate::decl(),
			])
			.state({
				let main_runtime_handle = main_runtime_handle.clone();
				let builder = builder.clone();
				move |state| {
					let builder = builder.clone();
					state.put(Arc::new(OpState {
						builder,
						main_runtime_handle: main_runtime_handle.clone(),
					}));
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
			extensions: vec![
				deno_webidl::init(),
				deno_url::init(),
				deno_web::init::<Permissions>(deno_web::BlobStore::default(), None),
				tangram_extension,
			],
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
			builder,
			_compiler: compiler,
			_main_runtime_handle: main_runtime_handle,
			runtime,
			inspector_session,
			context_id,
		};
		Ok(runtime)
	}

	#[allow(clippy::too_many_lines)]
	pub async fn js(&mut self, js: &Js) -> Result<Hash> {
		// Acquire a shared lock to the builder.
		let builder = self.builder.lock_shared().await?;

		// Create the URL.
		let url = js::Url::new_package_module(js.package, js.path.clone());

		// Load the module.
		let module_id = self
			.runtime
			.load_side_module(&url.clone().into(), None)
			.await?;
		let evaluate_receiver = self.runtime.mod_evaluate(module_id);
		self.runtime.run_event_loop(false).await?;
		evaluate_receiver.await.unwrap()?;

		// Move the args to v8.
		let args = builder
			.get_expression_local(js.args)
			.context("Failed to get the args expression.")?;
		let args = args.as_array().context("The args must be an array.")?;
		let mut arg_values = Vec::new();
		for arg in args {
			// Create a try catch scope to call Tangram.toJson.
			let mut scope = self.runtime.handle_scope();
			let mut try_catch_scope = v8::TryCatch::new(&mut scope);

			let arg = builder.get_expression_local(*arg)?;
			let arg = serde_v8::to_v8(&mut try_catch_scope, arg)
				.context("Failed to move the args expression to v8.")?;

			// Retrieve Tangram.fromJson.
			let from_json_function: v8::Local<v8::Function> =
				deno_core::JsRuntime::grab_global(&mut try_catch_scope, "Tangram.fromJson")
					.context("Failed to get Tangram.fromJson.")?;

			// Call Tangram.fromJson.
			let undefined = v8::undefined(&mut try_catch_scope);
			let output = from_json_function.call(&mut try_catch_scope, undefined.into(), &[arg]);

			// If an exception was caught, return an error with an error message.
			if try_catch_scope.has_caught() {
				let exception = try_catch_scope.exception().unwrap();
				let mut scope = v8::HandleScope::new(&mut try_catch_scope);
				let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
				bail!(error);
			}

			// If there was no caught exception then retrieve the return value.
			let output = output.unwrap();

			// Move the return value to the global scope.
			let output = v8::Global::new(&mut try_catch_scope, output);
			drop(try_catch_scope);
			drop(scope);

			// Resolve the value.
			let output = self.runtime.resolve_value(output).await?;

			arg_values.push(output);
		}

		// Retrieve the specified export from the module.
		let module_namespace = self.runtime.get_module_namespace(module_id)?;
		let mut scope = self.runtime.handle_scope();
		let module_namespace = v8::Local::<v8::Object>::new(&mut scope, module_namespace);
		let export_name = js.name.clone();
		let export_literal = v8::String::new(&mut scope, &export_name).unwrap();
		let export: v8::Local<v8::Function> = module_namespace
			.get(&mut scope, export_literal.into())
			.with_context(|| {
				format!(r#"Failed to get the export "{export_name}" from URL "{url}"."#)
			})?
			.try_into()
			.with_context(|| {
				format!(r#"The export "{export_name}" from URL "{url}" must be a function."#)
			})?;

		// Create a scope to call the export.
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		// Move the arg values to the try catch scope.
		let arg_values = arg_values
			.iter()
			.map(|arg| v8::Local::new(&mut try_catch_scope, arg))
			.collect::<Vec<_>>();

		// Call the specified export.
		let undefined = v8::undefined(&mut try_catch_scope);
		let output = export.call(&mut try_catch_scope, undefined.into(), &arg_values);

		// If an exception was caught, return an error with an error message.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
			bail!(error);
		}

		// If there was no caught exception then retrieve the return value.
		let output = output.unwrap();

		// Move the return value to the global scope.
		let output = v8::Global::new(&mut try_catch_scope, output);
		drop(try_catch_scope);
		drop(scope);

		// Resolve the output.
		let output = self.runtime.resolve_value(output).await?;

		// Create a try catch scope to call Tangram.toJson.
		let mut scope = self.runtime.handle_scope();
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		// Move the output to the try catch scope.
		let output = v8::Local::new(&mut try_catch_scope, output);

		// Retrieve Tangram.toJson.
		let to_json_function: v8::Local<v8::Function> =
			deno_core::JsRuntime::grab_global(&mut try_catch_scope, "Tangram.toJson")
				.context("Failed to get Tangram.toJson.")?;

		// Call Tangram.toJson.
		let undefined = v8::undefined(&mut try_catch_scope);
		let output = to_json_function.call(&mut try_catch_scope, undefined.into(), &[output]);

		// If an exception was caught, return an error with an error message.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
			bail!(error);
		}

		// If there was no caught exception then retrieve the return value.
		let output = output.unwrap();

		// Move the return value to the global scope.
		let output = v8::Global::new(&mut try_catch_scope, output);
		drop(try_catch_scope);
		drop(scope);

		// Resolve the output.
		let output = self.runtime.resolve_value(output).await?;

		// Deserialize the output.
		let mut scope = self.runtime.handle_scope();
		let output = v8::Local::new(&mut scope, output);
		let expression: Expression = serde_v8::from_v8(&mut scope, output)?;
		drop(scope);

		// Add the expression.
		let hash = builder.add_expression(&expression).await?;

		Ok(hash)
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
			function stringify(value) {
				return Tangram.stringify(value);
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
	op(state, |state| async move {
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
	op(state, |state| async move {
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
	op(state, |state| async move {
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
	op(state, |state| async move {
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
	op(state, |state| async move {
		let output = state
			.builder
			.lock_shared()
			.await?
			.evaluate(hash, hash)
			.await?;
		Ok::<_, anyhow::Error>(output)
	})
	.await
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
	let main_runtime_handle = state.main_runtime_handle.clone();
	let output = main_runtime_handle.spawn(f(state)).await.unwrap()?;
	Ok(output)
}

struct Permissions;

impl deno_web::TimersPermission for Permissions {
	fn allow_hrtime(&mut self) -> bool {
		false
	}

	fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
		// No-op.
	}
}
