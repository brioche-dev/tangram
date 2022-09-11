use self::module_loader::{ModuleLoader, TANGRAM_MODULE_SCHEME};
use crate::{
	artifact::Artifact,
	expression::{self, Expression},
	hash::Hash,
	lockfile::Lockfile,
	object,
	server::{repl::Output, Server},
};
use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use deno_core::{serde_v8, v8};
use itertools::Itertools;
use std::{cell::RefCell, collections::BTreeMap, convert::TryInto, fmt::Write, rc::Rc, sync::Arc};
use url::Url;

mod cdp;
mod module_loader;

#[derive(Debug)]
pub struct Runtime {
	task: tokio::task::JoinHandle<()>,
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
	Repl(Output),
	Run(Result<Expression>),
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/snapshot"));

/// This is the deno op state for the tangram deno extension.
#[derive(Clone)]
struct OpState {
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
		let task = server.local_pool_handle.spawn_pinned({
			let server = Arc::clone(server);
			move || js_runtime_task(server, main_runtime_handle, receiver)
		});

		Runtime { task, sender }
	}

	pub async fn repl(&self, code: String) -> Result<Output> {
		let request = Request::Repl { code };
		let response = self.request(request).await?;
		let response = if let Response::Repl(response) = response {
			response
		} else {
			bail!("Unexpected response type.")
		};
		Ok(response)
	}

	pub async fn run(&self, process: &expression::JsProcess) -> Result<Result<Expression>> {
		// TODO Evaluate the module expression.
		let request = Request::Run {
			process: process.clone(),
		};
		let response = self.request(request).await?;
		let response = if let Response::Run(response) = response {
			response
		} else {
			bail!("Unexpected response type.")
		};
		Ok(response)
	}

	async fn request(&self, request: Request) -> Result<Response> {
		// Create a channel to send the request and receive the response.
		let (sender, receiver) = tokio::sync::oneshot::channel();

		// Send the request.
		match self.sender.send((request, sender)) {
			Ok(_) => {},
			Err(_) => bail!("Failed to send a request to the js task."),
		};

		// Handle an error receiving the response.
		let response = match receiver.await {
			Ok(response) => response,
			Err(_) => bail!("Failed to receive a response from the js task."),
		};

		Ok(response)
	}
}

impl Drop for Runtime {
	fn drop(&mut self) {
		self.task.abort();
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
			op_tangram_artifact::decl(),
			op_tangram_console_log::decl(),
			op_tangram_create_artifact::decl(),
			op_tangram_dependency::decl(),
			op_tangram_directory::decl(),
			op_tangram_evaluate::decl(),
			op_tangram_fetch::decl(),
			op_tangram_file::decl(),
			op_tangram_path::decl(),
			op_tangram_process::decl(),
			op_tangram_symlink::decl(),
			op_tangram_target::decl(),
			op_tangram_template::decl(),
		])
		.state({
			let server = Arc::clone(&server);
			let main_runtime_handle = main_runtime_handle.clone();
			move |state| {
				state.put(Arc::new(OpState {
					server: Arc::clone(&server),
					main_runtime_handle: main_runtime_handle.clone(),
				}));
				Ok(())
			}
		})
		.build();

	// Create the module loader.
	let module_loader = Rc::new(ModuleLoader::new(
		Arc::clone(&server),
		main_runtime_handle.clone(),
	));

	// Create the js runtime.
	let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
		source_map_getter: Some(
			Box::new(Rc::clone(&module_loader)) as Box<dyn deno_core::SourceMapGetter>
		),
		module_loader: Some(Rc::clone(&module_loader) as Rc<dyn deno_core::ModuleLoader>),
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
			// Handle a repl request.
			Request::Repl { code } => {
				let response =
					handle_repl_request(&mut js_runtime, &mut inspector_session, context_id, code)
						.await;
				let response = Response::Repl(response);
				sender.send(response).unwrap();
			},

			// Handle a run request.
			Request::Run { process } => {
				let response = handle_run_request(&mut js_runtime, &module_loader, process).await;
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
) -> Output {
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
			return Output::Error {
				message: error.to_string(),
			};
		},
	};

	// If there was an error, return its description.
	if let Some(exception_details) = evaluate_response.exception_details {
		return Output::Error {
			message: exception_details.exception.unwrap().description.unwrap(),
		};
	}

	// If the evaluation produced a value, return it.
	if let Some(value) = evaluate_response.result.value {
		let output = serde_json::to_string_pretty(&value).unwrap();
		return Output::Success {
			message: Some(output),
		};
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
			return Output::Error {
				message: error.to_string(),
			};
		},
	};

	// If there was an error, return its description.
	if let Some(exception_details) = call_function_on_response.exception_details {
		return Output::Error {
			message: exception_details.exception.unwrap().description.unwrap(),
		};
	}

	// Retrieve the output.
	let output = if let Some(output) = call_function_on_response.result.value {
		output
	} else {
		return Output::Error {
			message: "An unexpected error occurred.".to_owned(),
		};
	};

	// Get the output as a string.
	let output = output.as_str().unwrap().to_owned();

	Output::Success {
		message: Some(output),
	}
}

async fn handle_run_request(
	js_runtime: &mut deno_core::JsRuntime,
	module_loader: &ModuleLoader,
	process: expression::JsProcess,
) -> Result<Expression> {
	// Get the path expression for the module.
	let module = match *process.module {
		Expression::Path(module) => module,
		_ => bail!("The module must be a path."),
	};

	// Get the module's artifact.
	let module_artifact = match *module.artifact {
		Expression::Artifact(artifact) => artifact,
		_ => bail!("Module artifact must be an artifact."),
	};

	// Create the module URL.
	let mut module_url = format!(
		"{TANGRAM_MODULE_SCHEME}://{}",
		module_artifact.object_hash(),
	);

	// Add the module path if necessary.
	if let Some(path) = module.path {
		module_url.push('/');
		module_url.push_str(path.as_str());
	}

	// Add the lockfile if necessary.
	if let Some(lockfile) = process.lockfile {
		let lockfile_hash = module_loader.add_lockfile(lockfile);
		write!(module_url, "?lockfile_hash={lockfile_hash}").unwrap();
	}

	// Parse the module URL.
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
			format!(
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
	let output = export.call(&mut try_catch_scope, undefined.into(), &args);

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

	// Run the event loop to completion.
	js_runtime.run_event_loop(false).await?;

	// Retrieve the return value.
	let mut scope = js_runtime.handle_scope();
	let output = v8::Local::new(&mut scope, output);
	let output = if output.is_promise() {
		let promise: v8::Local<v8::Promise> = output.try_into().unwrap();
		promise.result(&mut scope)
	} else {
		output
	};
	let output = serde_v8::from_v8(&mut scope, output)?;

	Ok(output)
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_artifact(object_hash: object::Hash) -> Result<Artifact, deno_core::error::AnyError> {
	Ok(Artifact::new(object_hash))
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
async fn op_tangram_create_artifact(
	state: Rc<RefCell<deno_core::OpState>>,
	object_hash: object::Hash,
) -> Result<Artifact, deno_core::error::AnyError> {
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let server = Arc::clone(&state.server);
			async move {
				let output = server.create_artifact(object_hash).await?;
				Ok::<_, anyhow::Error>(output)
			}
		})
		.await
		.unwrap()?;
	Ok(output)
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_dependency(
	state: Rc<RefCell<deno_core::OpState>>,
	artifact: Artifact,
) -> Result<object::Hash, deno_core::error::AnyError> {
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let server = Arc::clone(&state.server);
			async move {
				let output = server.add_dependency(artifact).await?;
				Ok::<_, anyhow::Error>(output)
			}
		})
		.await
		.unwrap()?;
	Ok(output)
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_directory(
	state: Rc<RefCell<deno_core::OpState>>,
	entries: BTreeMap<String, object::Hash>,
) -> Result<object::Hash, deno_core::error::AnyError> {
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let server = Arc::clone(&state.server);
			async move {
				let output = server.add_directory(entries).await?;
				Ok::<_, anyhow::Error>(output)
			}
		})
		.await
		.unwrap()?;
	Ok(output)
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_evaluate(
	state: Rc<RefCell<deno_core::OpState>>,
	expression: Expression,
) -> Result<Expression, deno_core::error::AnyError> {
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let server = Arc::clone(&state.server);
			async move {
				let output = server.evaluate(&expression, expression.hash()).await?;
				Ok::<_, anyhow::Error>(output)
			}
		})
		.await
		.unwrap()?;
	Ok(output)
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

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum FileBlob {
	String(String),
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FileOptions {
	executable: Option<bool>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_file(
	state: Rc<RefCell<deno_core::OpState>>,
	blob: FileBlob,
	_options: Option<FileOptions>,
) -> Result<object::Hash, deno_core::error::AnyError> {
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let server = Arc::clone(&state.server);
			async move {
				let blob = match blob {
					FileBlob::String(string) => server.add_blob(string.as_bytes()).await?,
				};
				let output = server.add_file(blob, false).await?;
				Ok::<_, anyhow::Error>(output)
			}
		})
		.await
		.unwrap()?;
	Ok(output)
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_path(
	artifact: Expression,
	path: Option<Utf8PathBuf>,
) -> Result<Expression, deno_core::error::AnyError> {
	Ok(Expression::Path(expression::Path {
		artifact: Box::new(artifact),
		path: path.map(Into::into),
	}))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_process(
	process: expression::Process,
) -> Result<Expression, deno_core::error::AnyError> {
	Ok(Expression::Process(process))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_symlink(
	state: Rc<RefCell<deno_core::OpState>>,
	target: Utf8PathBuf,
) -> Result<object::Hash, deno_core::error::AnyError> {
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let server = Arc::clone(&state.server);
			async move {
				let output = server.add_symlink(target).await?;
				Ok::<_, anyhow::Error>(output)
			}
		})
		.await
		.unwrap()?;
	Ok(output)
}

#[derive(serde::Deserialize)]
struct TargetArgs {
	lockfile: Option<Lockfile>,
	package: Artifact,
	name: String,
	#[serde(default)]
	args: Vec<Expression>,
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
		.map(|string| Expression::String(string.into()))
		.interleave(args.placeholders)
		.collect();
	let template = crate::expression::Template { components };
	Ok(Expression::Template(template))
}
