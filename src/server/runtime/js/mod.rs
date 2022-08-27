use self::module_loader::{ModuleLoader, TANGRAM_MODULE_SCHEME};
use crate::{
	artifact::Artifact,
	expression::{self, Expression},
	hash::Hash,
	lockfile::Lockfile,
	server::Server,
	value::Value,
};
use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use deno_core::{serde_v8, v8};
use itertools::Itertools;
use std::{cell::RefCell, fmt::Write, rc::Rc, sync::Arc};
use url::Url;

mod cdp;
mod module_loader;

#[derive(Debug)]
pub struct Runtime {
	sender: RequestSender,
}

type RequestSender =
	tokio::sync::mpsc::UnboundedSender<(Request, tokio::sync::oneshot::Sender<Response>)>;

type RequestReceiver =
	tokio::sync::mpsc::UnboundedReceiver<(Request, tokio::sync::oneshot::Sender<Response>)>;

#[derive(Debug)]
enum Request {
	Repl { code: String },
	Run { process: expression::JsProcess },
}

#[derive(Debug)]
enum Response {
	Repl(Result<Option<String>, String>),
	Run(Result<Expression>),
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/snapshot"));

/// This is the deno op state for the tangram deno extension.
#[derive(Clone)]
struct TangramOpState {
	server: Arc<Server>,
	main_runtime_handle: tokio::runtime::Handle,
}

impl Runtime {
	#[must_use]
	pub fn new(server: &Arc<Server>) -> Runtime {
		// Get a handle to the current tokio runtime.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Create a channel to send messages to the js task.
		let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();

		// Create the js task.
		server.local_pool_handle.spawn_pinned({
			let server = Arc::clone(server);
			move || js_runtime_task(server, main_runtime_handle, receiver)
		});

		Runtime { sender }
	}

	pub async fn repl(&self, code: String) -> Result<Result<Option<String>, String>> {
		let request = Request::Repl { code: code.clone() };
		let response = self.request(request).await?;
		let response = match response {
			Response::Repl(response) => response,
			_ => bail!("Unexpected response type."),
		};
		Ok(response)
	}

	pub async fn run(&self, process: expression::JsProcess) -> Result<Result<Expression>> {
		let request = Request::Run { process };
		let response = self.request(request).await?;
		let response = match response {
			Response::Run(response) => response,
			_ => bail!("Unexpected response type."),
		};
		Ok(response)
	}

	async fn request(&self, request: Request) -> Result<Response> {
		let (sender, receiver) = tokio::sync::oneshot::channel();
		match self.sender.send((request, sender)) {
			Ok(_) => {},
			Err(_) => bail!("Failed to send a request to the js task.".to_owned()),
		};
		let response = match receiver.await {
			Ok(response) => response,
			Err(_) => bail!("Failed to receive a response from the js task.".to_owned()),
		};
		Ok(response)
	}
}

async fn js_runtime_task(
	server: Arc<Server>,
	main_runtime_handle: tokio::runtime::Handle,
	mut receiver: RequestReceiver,
) {
	// Build the tangram extension.
	let tangram_extension = deno_core::Extension::builder()
		.ops(vec![
			op_tangram_console_log::decl(),
			op_tangram_evaluate::decl(),
			op_tangram_fetch::decl(),
			op_tangram_path::decl(),
			op_tangram_process::decl(),
			op_tangram_target::decl(),
			op_tangram_template::decl(),
		])
		.state({
			let server = Arc::clone(&server);
			let main_runtime_handle = main_runtime_handle.clone();
			move |state| {
				state.put(TangramOpState {
					server: Arc::clone(&server),
					main_runtime_handle: main_runtime_handle.clone(),
				});
				Ok(())
			}
		})
		.build();

	// Create the js runtime.
	let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
		module_loader: Some(Rc::new(ModuleLoader::new(
			Arc::clone(&server),
			main_runtime_handle.clone(),
		))),
		extensions: vec![tangram_extension],
		startup_snapshot: Some(deno_core::Snapshot::Static(SNAPSHOT)),
		..Default::default()
	});

	// Create the v8 inspector session.
	let mut inspector_session = js_runtime.inspector().create_local_session();

	// Enable the inspector runtime.
	futures::try_join!(
		inspector_session.post_message::<()>("Runtime.enable", None),
		js_runtime.run_event_loop(false),
	)
	.unwrap();

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

	// Respond to each message on the receiver.
	while let Some((request, sender)) = receiver.recv().await {
		match request {
			Request::Repl { code } => {
				let response =
					handle_repl_request(&mut js_runtime, &mut inspector_session, context_id, code)
						.await;
				let response = Response::Repl(response);
				sender.send(response).unwrap();
			},
			Request::Run { process } => {
				let response = handle_run_request(&server, &mut js_runtime, process).await;
				let response = Response::Run(response);
				sender.send(response).unwrap();
			},
		};
	}
}

#[allow(clippy::too_many_lines)]
async fn handle_repl_request(
	js_runtime: &mut deno_core::JsRuntime,
	inspector_session: &mut deno_core::LocalInspectorSession,
	context_id: u64,
	code: String,
) -> Result<Option<String>, String> {
	// If the code begins with an open curly and does not end in a semicolon, wrap it in parens to make it an ExpressionStatement instead of a BlockStatement.
	let code = if code.trim_start().starts_with('{') && !code.trim_end().ends_with(';') {
		format!("({code})")
	} else {
		code
	};

	// Evaluate the code.
	let evaluate_response: cdp::EvaluateResponse = match futures::try_join!(
		inspector_session.post_message(
			"Runtime.evaluate",
			Some(cdp::EvaluateArgs {
				context_id: Some(context_id),
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
		js_runtime.run_event_loop(false),
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
			switch (typeof value) {
				case "object": {
					if (value instanceof Error) {
						return value.stack;
					} else if (value instanceof Promise) {
						return "Promise";
					} else if (value instanceof Array) {
						return `[ ${Object.values(value).map(value => stringify(value)).flat().join(", ")} ]`;
					} else {
						return `{ ${Object.entries(value).map(([key, value]) => `${key}: ${stringify(value)}`).join(", ")} }`;
					}
				}
				case "function": {
					return `[Function: ${value.name || "(anonymous)"}]`;
				}
				case "undefined": {
					return "undefined";
				}
				default: {
					return JSON.stringify(value);
				}
			}
		}
	"#;
	let call_function_on_response: cdp::CallFunctionOnResponse = match futures::try_join!(
		inspector_session.post_message(
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
				execution_context_id: Some(context_id),
				object_group: None,
				throw_on_side_effect: None
			}),
		),
		js_runtime.run_event_loop(false),
	) {
		Ok((response, _)) => serde_json::from_value(response).unwrap(),
		Err(error) => {
			return Err(error.to_string());
		},
	};

	// If there was an error, return its description.
	if let Some(exception_details) = call_function_on_response.exception_details {
		return Err(exception_details.exception.unwrap().description.unwrap());
	}

	// Retrieve the output.
	let value = if let Some(value) = call_function_on_response.result.value {
		value
	} else {
		return Err("An unexpected error occurred.".to_owned());
	};

	// Get the output as a string.
	let output = value.as_str().unwrap().to_owned();

	Ok(Some(output))
}

async fn handle_run_request(
	server: &Arc<Server>,
	js_runtime: &mut deno_core::JsRuntime,
	process: expression::JsProcess,
) -> Result<Expression> {
	// Evaluate the module expression to get a path value.
	let module = server.evaluate(*process.module).await?;
	let module = match module {
		Value::Path(module) => module,
		_ => bail!("Module must be a path."),
	};

	let mut module_url = format!(
		"{TANGRAM_MODULE_SCHEME}://{}",
		module.artifact.object_hash()
	);
	if let Some(path) = module.path {
		module_url.push('/');
		module_url.push_str(path.as_str());
	}
	let module_url = Url::parse(&module_url).unwrap();

	// Load the module.
	let module_id = js_runtime.load_side_module(&module_url, None).await?;
	let evaluate_receiver = js_runtime.mod_evaluate(module_id);
	js_runtime.run_event_loop(false).await?;
	evaluate_receiver.await.unwrap()?;

	// Retrieve the specified export from the module.
	let module_namespace = js_runtime.get_module_namespace(module_id)?;
	let mut scope = js_runtime.handle_scope();
	let module_namespace = v8::Local::<v8::Object>::new(&mut scope, module_namespace);
	let export_name = process.export;
	let export_literal = v8::String::new(&mut scope, &export_name).unwrap();
	let export: v8::Local<v8::Function> = module_namespace
		.get(&mut scope, export_literal.into())
		.ok_or_else(|| {
			anyhow!(r#"Failed to get the export "{export_name}" from the module "{module_url}"."#)
		})?
		.try_into()
		.with_context(|| {
			anyhow!(
				r#"The export "{export_name}" from the module "{module_url}" must be a function."#
			)
		})?;

	// Create a scope to call the export.
	let mut try_catch_scope = v8::TryCatch::new(&mut scope);
	let undefined = v8::undefined(&mut try_catch_scope);

	// Evaluate the args and move them to v8.
	let args = process
		.args
		.into_iter()
		.map(|arg| {
			let arg = serde_v8::to_v8(&mut try_catch_scope, arg)?;
			Ok(arg)
		})
		.collect::<Result<Vec<_>>>()?;

	// Call the specified export.
	let value = export.call(&mut try_catch_scope, undefined.into(), &args);
	if try_catch_scope.has_caught() {
		let exception = try_catch_scope.exception().unwrap();
		let mut scope = v8::HandleScope::new(&mut try_catch_scope);
		let exception_string = exception_to_string(&mut scope, exception);
		bail!("{}", exception_string);
	}
	let value = value.unwrap();

	// Move the return value to the global scope.
	let value = v8::Global::new(&mut try_catch_scope, value);
	drop(try_catch_scope);
	drop(scope);

	// Run the event loop to completion.
	js_runtime.run_event_loop(false).await?;

	// Retrieve the return value.
	let mut scope = js_runtime.handle_scope();
	let value = v8::Local::new(&mut scope, value);
	let value = if value.is_promise() {
		let promise: v8::Local<v8::Promise> = value.try_into().unwrap();
		promise.result(&mut scope)
	} else {
		value
	};
	let value = serde_v8::from_v8(&mut scope, value)?;

	Ok(value)
}

/// Render an exception to a string. The string will include the exception's message and a stack trace with source maps applied.
fn exception_to_string(scope: &mut v8::HandleScope, exception: v8::Local<v8::Value>) -> String {
	let mut string = String::new();
	let message = exception
		.to_string(scope)
		.unwrap()
		.to_rust_string_lossy(scope);
	writeln!(&mut string, "{}", message).unwrap();
	let stack_trace = v8::Exception::get_stack_trace(scope, exception).unwrap();
	for i in 0..stack_trace.get_frame_count() {
		let stack_trace_frame = stack_trace.get_frame(scope, i).unwrap();
		let source_url = Url::parse(
			&stack_trace_frame
				.get_script_name(scope)
				.unwrap()
				.to_rust_string_lossy(scope),
		)
		.unwrap();
		let source_line = stack_trace_frame.get_line_number();
		let source_column = stack_trace_frame.get_column();
		// if let Some(source_map) = module_handle.source_map.as_ref() {
		// 	let token = source_map
		// 		.lookup_token((source_line - 1).into(), (source_column - 1).into())
		// 		.unwrap();
		// 	source_url = token.get_source().unwrap_or("<unknown>");
		// 	source_line = token.get_src_line() + 1;
		// 	source_column = token.get_src_col() + 1;
		// }
		write!(
			&mut string,
			"{}:{}:{}",
			source_url, source_line, source_column,
		)
		.unwrap();
		if i < stack_trace.get_frame_count() - 1 {
			writeln!(&mut string).unwrap();
		}
	}
	string
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_console_log(args: Vec<serde_json::Value>) -> Result<(), deno_core::error::AnyError> {
	let len = args.len();
	for (i, arg) in args.into_iter().enumerate() {
		print!("{arg}");
		if i != len - 1 {
			print!(" ");
		}
	}
	println!();
	Ok(())
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_evaluate(
	state: Rc<RefCell<deno_core::OpState>>,
	expression: Expression,
) -> Result<Value, deno_core::error::AnyError> {
	let (server, main_runtime_handle) = {
		let state = state.borrow();
		let tangram_state = state.borrow::<TangramOpState>();
		let server = Arc::clone(&tangram_state.server);
		let main_runtime_handle = tangram_state.main_runtime_handle.clone();
		(server, main_runtime_handle)
	};
	let task = async move {
		let value = server.evaluate(expression).await?;
		Ok::<_, anyhow::Error>(value)
	};
	let value = main_runtime_handle.spawn(task).await.unwrap()?;
	Ok(value)
}

#[derive(serde::Deserialize)]
struct FetchArgs {
	url: Url,
	hash: Option<Hash>,
	unpack: bool,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_fetch(args: FetchArgs) -> Result<Expression, deno_core::error::AnyError> {
	Ok(Expression::Fetch(crate::expression::Fetch {
		url: args.url,
		hash: args.hash,
		unpack: args.unpack,
	}))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_path(
	artifact: Expression,
	path: Option<Utf8PathBuf>,
) -> Result<Expression, deno_core::error::AnyError> {
	Ok(Expression::Path(expression::Path {
		artifact: Box::new(artifact),
		path,
	}))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_process(
	process: expression::Process,
) -> Result<Expression, deno_core::error::AnyError> {
	Ok(Expression::Process(process))
}

#[derive(serde::Deserialize)]
struct TargetArgs {
	lockfile: Lockfile,
	package: Artifact,
	name: String,
	#[serde(default)]
	args: Vec<Box<Expression>>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_target(args: TargetArgs) -> Result<Expression, deno_core::error::AnyError> {
	Ok(Expression::Target(crate::expression::Target {
		lockfile: args.lockfile,
		package: args.package,
		name: args.name,
		args: args.args,
	}))
}

#[derive(serde::Deserialize)]
struct TemplateArgs {
	strings: Vec<String>,
	placeholders: Vec<Expression>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_template(args: TemplateArgs) -> Result<Expression, deno_core::error::AnyError> {
	let components = args
		.strings
		.into_iter()
		.map(Expression::String)
		.interleave(args.placeholders)
		.collect();
	let template = crate::expression::Template { components };
	Ok(Expression::Template(template))
}
