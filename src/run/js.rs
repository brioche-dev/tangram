use self::{
	convert::{from_v8, ToV8},
	syscall::syscall,
};
use crate::{
	language::{self, Import, Module, Position},
	run, Client, Error, Package, Result, Server, Task, Value, WrapErr,
};
use futures::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::{
	cell::RefCell, future::poll_fn, num::NonZeroI32, rc::Rc, str::FromStr, sync::Arc, task::Poll,
};

mod convert;
mod syscall;

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/global.heapsnapshot"));

const SOURCE_MAP: &[u8] =
	include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/global.js.map"));

struct State {
	client: Client,
	futures: Rc<RefCell<Futures>>,
	global_source_map: Option<SourceMap>,
	loaded_modules: Rc<RefCell<Vec<LoadedModule>>>,
	main_runtime_handle: tokio::runtime::Handle,
	run: Arc<run::State>,
}

type Futures = FuturesUnordered<
	LocalBoxFuture<'static, (Result<Box<dyn ToV8>>, v8::Global<v8::PromiseResolver>)>,
>;

struct LoadedModule {
	module: Module,
	v8_identity_hash: NonZeroI32,
	v8_module: v8::Global<v8::Module>,
	text: String,
	transpiled_text: Option<String>,
	source_map: Option<SourceMap>,
}

impl Server {
	pub async fn run_js(&self, task: &Task, state: &Arc<run::State>) -> Result<Option<Value>> {
		// Build the target on the server's local pool because it is a `!Send` future.
		let output = self
			.state
			.local_pool
			.spawn_pinned({
				let server = self.clone();
				let task = task.clone();
				let state = state.clone();
				let main_runtime_handle = tokio::runtime::Handle::current();
				move || async move { server.run_js_inner(task, state, main_runtime_handle).await }
			})
			.await
			.wrap_err("Failed to join the task.")??;

		Ok(output)
	}

	#[allow(clippy::await_holding_refcell_ref, clippy::too_many_lines)]
	async fn run_js_inner(
		&self,
		task: Task,
		state: Arc<run::State>,
		main_runtime_handle: tokio::runtime::Handle,
	) -> Result<Option<Value>> {
		let client = Client::with_server(self.clone());

		// Create the isolate params.
		let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);

		// Create the isolate.
		let mut isolate = v8::Isolate::new(params);

		// Set the host import module dynamically callback.
		isolate
			.set_host_import_module_dynamically_callback(host_import_module_dynamically_callback);

		// Set the host initialize import meta object callback.
		isolate.set_host_initialize_import_meta_object_callback(
			host_initialize_import_meta_object_callback,
		);

		// Create and enter the context.
		let mut handle_scope = v8::HandleScope::new(&mut isolate);
		let context = v8::Context::new(&mut handle_scope);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Create the state.
		let state = Rc::new(State {
			main_runtime_handle,
			client,
			run: state,
			global_source_map: Some(SourceMap::from_slice(SOURCE_MAP).unwrap()),
			loaded_modules: Rc::new(RefCell::new(Vec::new())),
			futures: Rc::new(RefCell::new(FuturesUnordered::new())),
		});

		// Set the state on the context.
		context.set_slot(&mut context_scope, state.clone());

		// Create the syscall function.
		let syscall_string =
			v8::String::new_external_onebyte_static(&mut context_scope, "syscall".as_bytes())
				.unwrap();
		let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
		let global = context.global(&mut context_scope);
		global
			.set(&mut context_scope, syscall_string.into(), syscall.into())
			.unwrap();

		// Get the tg global.
		let global = context.global(&mut context_scope);
		let tg =
			v8::String::new_external_onebyte_static(&mut context_scope, "tg".as_bytes()).unwrap();
		let tg = global.get(&mut context_scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		// Get the main function.
		let main =
			v8::String::new_external_onebyte_static(&mut context_scope, "main".as_bytes()).unwrap();
		let main = tg.get(&mut context_scope, main.into()).unwrap();
		let main = v8::Local::<v8::Function>::try_from(main).unwrap();

		// Call the main function.
		let undefined = v8::undefined(&mut context_scope);
		let task = task.to_v8(&mut context_scope).unwrap();
		let output = main
			.call(&mut context_scope, undefined.into(), &[task])
			.unwrap();

		// Make the output and context global.
		let output = v8::Global::new(&mut context_scope, output);
		let context = v8::Global::new(&mut context_scope, context);

		// Exit the context.
		drop(context_scope);
		drop(handle_scope);

		// Await the output.
		let output = await_value(&mut isolate, context.clone(), state.clone(), output).await?;

		// Enter the context.
		let mut handle_scope = v8::HandleScope::new(&mut isolate);
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Move the output to the context.
		let output = v8::Local::new(&mut context_scope, output);

		// Move the output from v8.
		let output = from_v8(&mut context_scope, output)?;

		// Exit the context.
		drop(context_scope);
		drop(handle_scope);

		Ok(output)
	}
}

/// Implement V8's dynamic import callback.
fn host_import_module_dynamically_callback<'s>(
	scope: &mut v8::HandleScope<'s>,
	_host_defined_options: v8::Local<'s, v8::Data>,
	resource_name: v8::Local<'s, v8::Value>,
	specifier: v8::Local<'s, v8::String>,
	_import_assertions: v8::Local<'s, v8::FixedArray>,
) -> Option<v8::Local<'s, v8::Promise>> {
	// Get the resource name.
	let resource_name = resource_name.to_string(scope).unwrap();
	let resource_name = resource_name.to_rust_string_lossy(scope);

	// Get the module.
	let module = if resource_name == "[global]" {
		let module = specifier.to_rust_string_lossy(scope);
		match Module::from_str(&module) {
			Ok(module) => module,
			Err(error) => {
				return None;
			},
		}
	} else {
		// Get the module.
		let module = match Module::from_str(&resource_name) {
			Ok(module) => module,
			Err(error) => {
				return None;
			},
		};

		// Get the import.
		let import = specifier.to_rust_string_lossy(scope);
		let import = match Import::from_str(&import) {
			Ok(import) => import,
			Err(error) => {
				let exception = v8::String::new(scope, "Invalid import specifier.")
					.unwrap()
					.into();
				scope.throw_exception(exception);
				return None;
			},
		};

		match resolve_module(scope, &module, &import) {
			Ok(module) => module,
			Err(error) => {
				return None;
			},
		}
	};

	// Load the module.
	let module = match load_module(scope, &module) {
		Ok(module) => module,
		Err(error) => {
			return None;
		},
	};

	// Instantiate the module.
	let output = module.instantiate_module(scope, resolve_module_callback);
	if output.is_none() {
		return None;
	};

	// Evaluate the module.
	let Some(output) = module.evaluate(scope) else {
		return None;
	};

	let promise = v8::Local::<v8::Promise>::try_from(output).unwrap();

	Some(promise)
}

/// Implement V8's module resolution callback.
fn resolve_module_callback<'s>(
	context: v8::Local<'s, v8::Context>,
	specifier: v8::Local<'s, v8::String>,
	import_assertions: v8::Local<'s, v8::FixedArray>,
	referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {
	let mut scope = unsafe { v8::CallbackScope::new(context) };
	match resolve_module_callback_inner(context, specifier, import_assertions, referrer) {
		Ok(value) => Some(value),
		Err(error) => {
			let exception = error
				.to_v8(&mut scope)
				.expect("Failed to serialize the error.");
			scope.throw_exception(exception);
			None
		},
	}
}

#[allow(clippy::too_many_lines)]
fn resolve_module_callback_inner<'s>(
	context: v8::Local<'s, v8::Context>,
	specifier: v8::Local<'s, v8::String>,
	_import_assertions: v8::Local<'s, v8::FixedArray>,
	referrer: v8::Local<'s, v8::Module>,
) -> Result<v8::Local<'s, v8::Module>> {
	// Get a scope for the callback.
	let mut scope = unsafe { v8::CallbackScope::new(context) };

	// Get the state.
	let state = context.get_slot::<Rc<State>>(&mut scope).unwrap().clone();

	// Get the module.
	let identity_hash = referrer.get_identity_hash();
	let module = state
		.loaded_modules
		.borrow()
		.iter()
		.find(|module| module.v8_identity_hash == identity_hash)
		.wrap_err_with(|| {
			format!(r#"Unable to find the module with identity hash "{identity_hash}"."#)
		})?
		.module
		.clone();

	// Get the import.
	let specifier = specifier.to_rust_string_lossy(&mut scope);
	let import = Import::from_str(&specifier).wrap_err("Failed to parse the import.")?;

	// Resolve the module.
	let module =
		resolve_module(&mut scope, &module, &import).wrap_err("Failed to resolve the module.")?;

	// Load the module.
	let module = load_module(&mut scope, &module).wrap_err("Failed to load the module.")?;

	Ok(module)
}

/// Resolve a module.
fn resolve_module(scope: &mut v8::HandleScope, module: &Module, import: &Import) -> Result<Module> {
	let context = scope.get_current_context();
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();
	let (sender, receiver) = std::sync::mpsc::channel();
	state.main_runtime_handle.spawn({
		let client = state.client.clone();
		let module = module.clone();
		let import = import.clone();
		async move {
			let module = module.resolve(&client, None, &import).await;
			sender.send(module).unwrap();
		}
	});
	let module = receiver
		.recv()
		.unwrap()
		.wrap_err_with(|| format!(r#"Failed to resolve "{import}" relative to "{module}"."#))?;
	Ok(module)
}

/// Load a module.
fn load_module<'s>(
	scope: &mut v8::HandleScope<'s>,
	module: &Module,
) -> Result<v8::Local<'s, v8::Module>> {
	let context = scope.get_current_context();
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();

	// Return a cached module if this module has already been loaded.
	if let Some(module) = state
		.loaded_modules
		.borrow()
		.iter()
		.find(|cached_module| &cached_module.module == module)
	{
		let module = v8::Local::new(scope, &module.v8_module);
		return Ok(module);
	}

	// Define the module's origin.
	let resource_name = v8::String::new(scope, &module.to_string()).unwrap();
	let resource_line_offset = 0;
	let resource_column_offset = 0;
	let resource_is_shared_cross_origin = false;
	let script_id = state.loaded_modules.borrow().len().to_i32().unwrap() + 1;
	let source_map_url = v8::undefined(scope).into();
	let resource_is_opaque = true;
	let is_wasm = false;
	let is_module = true;
	let origin = v8::ScriptOrigin::new(
		scope,
		resource_name.into(),
		resource_line_offset,
		resource_column_offset,
		resource_is_shared_cross_origin,
		script_id,
		source_map_url,
		resource_is_opaque,
		is_wasm,
		is_module,
	);

	// Load the module.
	let (sender, receiver) = std::sync::mpsc::channel();
	state.main_runtime_handle.spawn({
		let client = state.client.clone();
		let module = module.clone();
		async move {
			let result = module.load(&client, None).await;
			sender.send(result).unwrap();
		}
	});
	let text = receiver
		.recv()
		.unwrap()
		.wrap_err_with(|| format!(r#"Failed to load module "{module}"."#))?;

	// Transpile the module.
	let language::transpile::Output {
		transpiled_text,
		source_map,
	} = Module::transpile(text.clone()).wrap_err("Failed to transpile the module.")?;
	let source_map =
		SourceMap::from_slice(source_map.as_bytes()).wrap_err("Failed to parse the source map.")?;

	// Compile the module.
	let mut try_catch_scope = v8::TryCatch::new(scope);
	let source = v8::String::new(&mut try_catch_scope, &transpiled_text).unwrap();
	let source = v8::script_compiler::Source::new(source, Some(&origin));
	let v8_module = v8::script_compiler::compile_module(&mut try_catch_scope, source);
	let Some(v8_module) = v8_module else {
		let exception = try_catch_scope.exception().unwrap();
		let error = error_from_exception(&mut try_catch_scope, &state, exception);
		return Err(error);
	};
	drop(try_catch_scope);

	// Cache the module.
	state.loaded_modules.borrow_mut().push(LoadedModule {
		v8_identity_hash: v8_module.get_identity_hash(),
		v8_module: v8::Global::new(scope, v8_module),
		module: module.clone(),
		text,
		transpiled_text: Some(transpiled_text),
		source_map: Some(source_map),
	});

	Ok(v8_module)
}

/// Implement V8's import.meta callback.
extern "C" fn host_initialize_import_meta_object_callback(
	context: v8::Local<v8::Context>,
	module: v8::Local<v8::Module>,
	meta: v8::Local<v8::Object>,
) {
	// Create the scope.
	let mut scope = unsafe { v8::CallbackScope::new(context) };

	// Get the state.
	let state = context.get_slot::<Rc<State>>(&mut scope).unwrap().clone();

	// Get the module.
	let identity_hash = module.get_identity_hash();
	let module = state
		.loaded_modules
		.borrow()
		.iter()
		.find(|module| module.v8_identity_hash == identity_hash)
		.unwrap()
		.module
		.clone();
	let module = module.unwrap_normal_ref();

	// Create the module object.
	let object = v8::Object::new(&mut scope);

	let key = v8::String::new_external_onebyte_static(&mut scope, "package".as_bytes()).unwrap();
	let value = Package::with_id(module.package).to_v8(&mut scope).unwrap();
	object.set(&mut scope, key.into(), value);

	let key = v8::String::new_external_onebyte_static(&mut scope, "path".as_bytes()).unwrap();
	let value = module.path.to_v8(&mut scope).unwrap();
	object.set(&mut scope, key.into(), value);

	// Set import.meta.module.
	let module_string =
		v8::String::new_external_onebyte_static(&mut scope, "module".as_bytes()).unwrap();
	meta.set(&mut scope, module_string.into(), object.into())
		.unwrap();
}

async fn await_value(
	isolate: &mut v8::OwnedIsolate,
	context: v8::Global<v8::Context>,
	state: Rc<State>,
	value: v8::Global<v8::Value>,
) -> Result<v8::Global<v8::Value>> {
	poll_fn(move |cx| await_value_inner(isolate, context.clone(), &state, value.clone(), cx)).await
}

fn await_value_inner(
	isolate: &mut v8::OwnedIsolate,
	context: v8::Global<v8::Context>,
	state: &State,
	value: v8::Global<v8::Value>,
	cx: &mut std::task::Context<'_>,
) -> Poll<Result<v8::Global<v8::Value>>> {
	// Poll the context's futures and resolve or reject all that are ready.
	loop {
		// Poll the context's futures.
		let (result, promise_resolver) = match state.futures.borrow_mut().poll_next_unpin(cx) {
			Poll::Ready(Some(output)) => output,
			Poll::Ready(None) => break,
			Poll::Pending => return Poll::Pending,
		};

		// Enter the context.
		let mut handle_scope = v8::HandleScope::new(isolate);
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Resolve or reject the promise.
		let promise_resolver = v8::Local::new(&mut context_scope, promise_resolver);
		match result.and_then(|value| value.to_v8(&mut context_scope)) {
			Ok(value) => {
				// Resolve the promise.
				promise_resolver.resolve(&mut context_scope, value);
			},
			Err(error) => {
				// Reject the promise.
				let exception = error
					.to_v8(&mut context_scope)
					.expect("Failed to serialize the error.");
				promise_resolver.reject(&mut context_scope, exception);
			},
		};
	}

	// Enter the context.
	let mut handle_scope = v8::HandleScope::new(isolate);
	let context = v8::Local::new(&mut handle_scope, context);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Handle the value.
	let value = v8::Local::new(&mut context_scope, value);
	match v8::Local::<v8::Promise>::try_from(value) {
		Err(_) => {
			let value = v8::Global::new(&mut context_scope, value);
			Poll::Ready(Ok::<_, Error>(value))
		},

		Ok(promise) => match promise.state() {
			v8::PromiseState::Pending => Poll::Pending,

			v8::PromiseState::Fulfilled => {
				let value = promise.result(&mut context_scope);
				let value = v8::Global::new(&mut context_scope, value);
				Poll::Ready(Ok(value))
			},

			v8::PromiseState::Rejected => {
				let exception = promise.result(&mut context_scope);
				let error = error_from_exception(&mut context_scope, &state, exception);
				Poll::Ready(Err(error))
			},
		},
	}
}

#[allow(clippy::too_many_lines)]
fn error_from_exception(
	scope: &mut v8::HandleScope,
	state: &State,
	exception: v8::Local<v8::Value>,
) -> Error {
	#[derive(Debug, serde::Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct V8StackTrace {
		call_sites: Vec<V8CallSite>,
	}

	#[allow(dead_code, clippy::struct_excessive_bools)]
	#[derive(Debug, serde::Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct V8CallSite {
		type_name: Option<String>,
		function_name: Option<String>,
		method_name: Option<String>,
		file_name: Option<String>,
		line_number: Option<u32>,
		column_number: Option<u32>,
		is_eval: bool,
		is_native: bool,
		is_constructor: bool,
		is_async: bool,
		is_promise_all: bool,
		// is_promise_any: bool,
		promise_index: Option<u32>,
	}

	// Get the message.
	let message = v8::Exception::create_message(scope, exception)
		.get(scope)
		.to_rust_string_lossy(scope);

	// Get the location.
	let exception_message = v8::Exception::create_message(scope, exception);
	let resource_name = exception_message
		.get_script_resource_name(scope)
		.and_then(|resource_name| <v8::Local<v8::String>>::try_from(resource_name).ok())
		.map(|resource_name| resource_name.to_rust_string_lossy(scope));
	let line = exception_message
		.get_line_number(scope)
		.unwrap()
		.to_u32()
		.unwrap()
		- 1;
	let character = exception_message.get_start_column().to_u32().unwrap();
	let position = Position { line, character };
	let location = get_location(state, resource_name.as_deref(), position);

	// Get the stack trace.
	let stack_string = v8::String::new_external_onebyte_static(scope, "stack".as_bytes()).unwrap();
	let stack_trace = if let Some(stack) = exception
		.is_native_error()
		.then(|| exception.to_object(scope).unwrap())
		.and_then(|exception| exception.get(scope, stack_string.into()))
		.and_then(|value| serde_v8::from_v8::<V8StackTrace>(scope, value).ok())
	{
		let stack_frames = stack
			.call_sites
			.iter()
			.map(|call_site| {
				// Get the location.
				let file_name = call_site.file_name.as_deref();
				let line: u32 = call_site.line_number? - 1;
				let character: u32 = call_site.column_number?;
				let position = Position { line, character };
				let location = get_location(state, file_name, position)?;
				Some(location)
			})
			.map(|location| StackFrame { location })
			.collect();

		// Create the stack trace.
		Some(StackTrace { stack_frames })
	} else {
		None
	};

	// Get the source.
	let cause_string = v8::String::new_external_onebyte_static(scope, "cause".as_bytes()).unwrap();
	let source = if let Some(cause) = exception
		.is_native_error()
		.then(|| exception.to_object(scope).unwrap())
		.and_then(|exception| exception.get(scope, cause_string.into()))
		.and_then(|value| value.to_object(scope))
	{
		let error = error_from_exception(scope, state, cause.into());
		Some(Arc::new(error))
	} else {
		None
	};

	eprintln!("{message}");
	if let Some(location) = location {
		eprintln!("{location}");
	}
	if let Some(stack_trace) = stack_trace {
		eprintln!("{stack_trace}");
	}
	if let Some(source) = source {
		eprintln!("{source}");
	}

	// Create the error.
	Error::with_message(message)
}

fn get_location(state: &State, file_name: Option<&str>, position: Position) -> Option<Location> {
	if file_name.map_or(false, |resource_name| resource_name == "[global]") {
		// If the file name is "[global]", then create a location whose source is a module.

		// Apply the global source map if it is available.
		let location = if let Some(global_source_map) = state.global_source_map.as_ref() {
			let token = global_source_map
				.lookup_token(position.line, position.character)
				.unwrap();
			let path = token.get_source().unwrap();
			let path = path.strip_prefix("../").unwrap().to_owned();
			let position = Position {
				line: token.get_src_line(),
				character: token.get_src_col(),
			};
			Location {
				source: Source::Global(Some(path)),
				position,
			}
		} else {
			Location {
				source: Source::Global(None),
				position,
			}
		};

		Some(location)
	} else if let Some(module) = file_name.and_then(|resource_name| resource_name.parse().ok()) {
		// If the file name is a module, then create a location whose source is a module.

		// Apply a source map if one is available.
		let modules = state.loaded_modules.borrow();
		let position = if let Some(source_map) = modules
			.iter()
			.find(|source_map_module| source_map_module.module == module)
			.and_then(|source_map_module| source_map_module.source_map.as_ref())
		{
			let token = source_map
				.lookup_token(position.line, position.character)
				.unwrap();
			Position {
				line: token.get_src_line(),
				character: token.get_src_col(),
			}
		} else {
			position
		};

		// Create the location.
		Some(Location {
			source: Source::Module(module),
			position,
		})
	} else {
		// Otherwise, the location cannot be determined.
		None
	}
}

/// A stack trace.
#[derive(Clone, Debug)]
pub struct StackTrace {
	pub stack_frames: Vec<StackFrame>,
}

/// A stack frame.
#[derive(Clone, Debug)]
pub struct StackFrame {
	pub location: Option<Location>,
}

/// A source location.
#[derive(Clone, Debug)]
pub struct Location {
	pub source: Source,
	pub position: Position,
}

/// A source.
#[derive(Clone, Debug)]
pub enum Source {
	Global(Option<String>),
	Module(Module),
}

impl std::fmt::Display for StackTrace {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		for stack_frame in &self.stack_frames {
			writeln!(f)?;
			write!(f, "  {stack_frame}")?;
		}
		Ok(())
	}
}

impl std::fmt::Display for StackFrame {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(location) = &self.location {
			write!(f, "{location}")?;
		} else {
			write!(f, "[unknown]")?;
		}
		Ok(())
	}
}

impl std::fmt::Display for Location {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let source = &self.source;
		let line = self.position.line + 1;
		let character = self.position.character + 1;
		write!(f, "{source}:{line}:{character}")?;
		Ok(())
	}
}

impl std::fmt::Display for Source {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Source::Global(path) => {
				let path = path.as_deref().unwrap_or("[unknown]");
				write!(f, "global:{path}")?;
			},

			Source::Module(module) => {
				write!(f, "{module}")?;
			},
		}
		Ok(())
	}
}
