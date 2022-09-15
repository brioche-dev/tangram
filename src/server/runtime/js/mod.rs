use self::module_loader::{ModuleLoader, TANGRAM_MODULE_SCHEME};
use crate::{
	expression::{self, Artifact, Dependency, Directory, Expression, File, Symlink},
	hash::Hash,
	lockfile::Lockfile,
	server::Server,
};
use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use deno_core::{serde_v8, v8};
use futures::future::try_join_all;
use itertools::Itertools;
use std::{
	cell::RefCell, collections::BTreeMap, convert::TryInto, fmt::Write, future::Future, rc::Rc,
	sync::Arc,
};
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
	Repl(ReplOutput),
	Run(Result<Hash>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ReplOutput {
	#[serde(rename = "success")]
	Success { message: Option<String> },
	#[serde(rename = "error")]
	Error { message: String },
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

	pub async fn repl(&self, code: String) -> Result<ReplOutput> {
		let request = Request::Repl { code };
		let response = self.request(request).await?;
		let response = if let Response::Repl(response) = response {
			response
		} else {
			bail!("Unexpected response type.")
		};
		Ok(response)
	}

	pub async fn run(&self, process: &expression::JsProcess) -> Result<Result<Hash>> {
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
			// Expression ops.
			op_tangram_null::decl(),
			op_tangram_bool::decl(),
			op_tangram_number::decl(),
			op_tangram_string::decl(),
			op_tangram_artifact::decl(),
			op_tangram_directory::decl(),
			op_tangram_file::decl(),
			op_tangram_symlink::decl(),
			op_tangram_dependency::decl(),
			op_tangram_path::decl(),
			op_tangram_template::decl(),
			op_tangram_fetch::decl(),
			op_tangram_process::decl(),
			op_tangram_target::decl(),
			op_tangram_array::decl(),
			op_tangram_map::decl(),
			// Other ops.
			op_tangram_blob::decl(),
			op_tangram_console_log::decl(),
			op_tangram_evaluate::decl(),
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
				let response =
					handle_run_request(&mut js_runtime, &module_loader, &server, &process).await;
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
) -> ReplOutput {
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
			return ReplOutput::Error {
				message: error.to_string(),
			};
		},
	};

	// If there was an error, return its description.
	if let Some(exception_details) = evaluate_response.exception_details {
		return ReplOutput::Error {
			message: exception_details.exception.unwrap().description.unwrap(),
		};
	}

	// If the evaluation produced a value, return it.
	if let Some(value) = evaluate_response.result.value {
		let output = serde_json::to_string_pretty(&value).unwrap();
		return ReplOutput::Success {
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
			return ReplOutput::Error {
				message: error.to_string(),
			};
		},
	};

	// If there was an error, return its description.
	if let Some(exception_details) = call_function_on_response.exception_details {
		return ReplOutput::Error {
			message: exception_details.exception.unwrap().description.unwrap(),
		};
	}

	// Retrieve the output.
	let output = if let Some(output) = call_function_on_response.result.value {
		output
	} else {
		return ReplOutput::Error {
			message: "An unexpected error occurred.".to_owned(),
		};
	};

	// Get the output as a string.
	let output = output.as_str().unwrap().to_owned();

	ReplOutput::Success {
		message: Some(output),
	}
}

async fn handle_run_request(
	js_runtime: &mut deno_core::JsRuntime,
	module_loader: &ModuleLoader,
	server: &Arc<Server>,
	process: &expression::JsProcess,
) -> Result<Hash> {
	// Get the path expression for the module.
	let module = match server.try_get_expression(process.module).await?.unwrap() {
		Expression::Path(module) => module,
		_ => bail!("The module must be a path."),
	};

	// Get the module's artifact.
	let module_artifact = match server.try_get_expression(module.artifact).await?.unwrap() {
		Expression::Artifact(artifact) => artifact,
		_ => bail!("Module artifact must be an artifact."),
	};

	// Create the module URL.
	let mut module_url = format!("{TANGRAM_MODULE_SCHEME}://{}", module_artifact.hash);

	// Add the module path if necessary.
	if let Some(path) = module.path {
		module_url.push('/');
		module_url.push_str(path.as_str());
	}

	// Add the lockfile if necessary.
	if let Some(lockfile) = process.lockfile.as_ref() {
		let lockfile_hash = module_loader.add_lockfile(lockfile.clone());
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
	let export_name = process.export.clone();
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
		.iter()
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
async fn op_tangram_null(
	state: Rc<RefCell<deno_core::OpState>>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Null).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_bool(
	state: Rc<RefCell<deno_core::OpState>>,
	value: bool,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Bool(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_number(
	state: Rc<RefCell<deno_core::OpState>>,
	value: f64,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Number(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_string(
	state: Rc<RefCell<deno_core::OpState>>,
	value: String,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::String(value.into()))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_artifact(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Artifact(Artifact { hash }))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_directory(
	state: Rc<RefCell<deno_core::OpState>>,
	entries: BTreeMap<String, Hash>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Directory(Directory { entries }))
			.await
	})
	.await
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
	options: Option<FileOptions>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		let blob_hash = match blob {
			FileBlob::String(string) => server.add_blob(string.as_bytes()).await?,
		};
		let executable = options
			.and_then(|options| options.executable)
			.unwrap_or(false);
		let output = server
			.add_expression(&Expression::File(File {
				blob_hash,
				executable,
			}))
			.await?;
		Ok(output)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_symlink(
	state: Rc<RefCell<deno_core::OpState>>,
	target: Utf8PathBuf,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Symlink(Symlink { target }))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_dependency(
	state: Rc<RefCell<deno_core::OpState>>,
	artifact: Artifact,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Dependency(Dependency { artifact }))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_path(
	state: Rc<RefCell<deno_core::OpState>>,
	artifact: Hash,
	path: Option<Utf8PathBuf>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Path(expression::Path {
				artifact,
				path: path.map(Into::into),
			}))
			.await
	})
	.await
}

#[derive(serde::Deserialize)]
struct TemplateArgs {
	strings: Vec<String>,
	placeholders: Vec<Hash>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_template(
	state: Rc<RefCell<deno_core::OpState>>,
	args: TemplateArgs,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		let components = try_join_all(args.strings.into_iter().map(|string| async {
			server
				.add_expression(&Expression::String(string.into()))
				.await
		}))
		.await?
		.into_iter()
		.interleave(args.placeholders)
		.collect();
		let template = crate::expression::Template { components };
		let output = server
			.add_expression(&Expression::Template(template))
			.await?;
		Ok(output)
	})
	.await
}

#[derive(serde::Deserialize)]
struct FetchArgs {
	url: Url,
	hash: Option<Hash>,
	unpack: bool,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_fetch(
	state: Rc<RefCell<deno_core::OpState>>,
	args: FetchArgs,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, move |server| async move {
		server
			.add_expression(&Expression::Fetch(crate::expression::Fetch {
				url: args.url,
				hash: args.hash,
				unpack: args.unpack,
			}))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_process(
	state: Rc<RefCell<deno_core::OpState>>,
	process: expression::Process,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, move |server| async move {
		server.add_expression(&Expression::Process(process)).await
	})
	.await
}

#[derive(serde::Deserialize)]
struct TargetArgs {
	lockfile: Option<Lockfile>,
	package: Artifact,
	name: String,
	#[serde(default)]
	args: Vec<Hash>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_target(
	state: Rc<RefCell<deno_core::OpState>>,
	args: TargetArgs,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Target(crate::expression::Target {
				lockfile: args.lockfile,
				package: args.package,
				name: args.name,
				args: args.args,
			}))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_array(
	state: Rc<RefCell<deno_core::OpState>>,
	value: Vec<Hash>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Array(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_map(
	state: Rc<RefCell<deno_core::OpState>>,
	value: BTreeMap<String, Hash>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		let value = value
			.into_iter()
			.map(|(key, value)| (key.into(), value))
			.collect();
		server.add_expression(&Expression::Map(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	value: String,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_blob(value.as_bytes()).await
	})
	.await
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
	hash: Hash,
) -> Result<Hash, deno_core::error::AnyError> {
	op(
		state,
		|server| async move { server.evaluate(hash, hash).await },
	)
	.await
}

async fn op<T, F, Fut>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<T, deno_core::error::AnyError>
where
	T: 'static + Send,
	F: FnOnce(Arc<Server>) -> Fut,
	Fut: 'static + Send + Future<Output = Result<T, deno_core::error::AnyError>>,
{
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let server = Arc::clone(&state.server);
			f(server)
		})
		.await
		.unwrap()?;
	Ok(output)
}
