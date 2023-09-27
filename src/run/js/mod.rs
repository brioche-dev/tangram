use self::convert::from_v8;
use crate::{
	module::{self, position::Position},
	Error, Id, Result, Server, Value, WrapErr,
};
use futures::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::{cell::RefCell, future::poll_fn, num::NonZeroI32, rc::Rc, sync::Arc, task::Poll};

mod convert;
mod syscall;

pub const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/global.heapsnapshot"));

pub const SOURCE_MAP: &[u8] =
	include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/global.js.map"));

pub struct State {
	pub global_source_map: Option<SourceMap>,
	pub modules: Rc<RefCell<Vec<Module>>>,
	pub futures: Rc<RefCell<FuturesUnordered<LocalBoxFuture<'static, FutureOutput>>>>,
}

#[derive(Debug)]
pub struct Module {
	pub module: module::Module,
	pub v8_identity_hash: NonZeroI32,
	pub v8_module: v8::Global<v8::Module>,
	pub text: String,
	pub transpiled_text: Option<String>,
	pub source_map: Option<SourceMap>,
}

pub struct FutureOutput {
	pub context: v8::Global<v8::Context>,
	pub promise_resolver: v8::Global<v8::PromiseResolver>,
	pub result: Result<v8::Global<v8::Value>>,
}

#[derive(serde::Serialize)]
struct Key {
	package: Id,
	path: String,
	name: String,
}

thread_local! {
	pub static THREAD_LOCAL_ISOLATE: Rc<RefCell<v8::OwnedIsolate>> = {
		// Create the isolate params.
		let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);

		// Create the isolate.
		let mut isolate = v8::Isolate::new(params);

		// Set the host initialize import meta object callback.
		isolate.set_host_initialize_import_meta_object_callback(host_initialize_import_meta_object_callback);

		Rc::new(RefCell::new(isolate))
	};
}

pub(crate) async fn build_inner(server: &Server) -> Result<Value> {
	// Build the target on the server's local pool because it is a `!Send` future.
	let output = server
		.state
		.local_pool
		.spawn_pinned({
			let server = server.clone();
			move || async move { target.build_inner_inner(server).await }
		})
		.await
		.map_err(Error::other)
		.wrap_err("Failed to join the task.")??;

	Ok(output)
}

#[allow(clippy::await_holding_refcell_ref, clippy::too_many_lines)]
async fn build_inner_inner(server: Server) -> Result<Value> {
	// Create the context.
	let context = create_context(server.clone());

	// Evaluate the module.
	let module = Module::Normal(module::Normal {
		package: self.package.id(),
		module_path: self.path.clone(),
	});
	super::module::evaluate(context.clone(), &module).await?;

	// Enter the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Local::new(&mut handle_scope, context.clone());
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Get the state.
	let state = context
		.get_slot::<Rc<State>>(&mut context_scope)
		.unwrap()
		.clone();

	// Create a try catch scope.
	let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
	let undefined = v8::undefined(&mut try_catch_scope);

	// Get the tg global.
	let global = context.global(&mut try_catch_scope);
	let tg =
		v8::String::new_external_onebyte_static(&mut try_catch_scope, "tg".as_bytes()).unwrap();
	let tg = global.get(&mut try_catch_scope, tg.into()).unwrap();
	let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

	// Get the targets.
	let targets =
		v8::String::new_external_onebyte_static(&mut try_catch_scope, "targets".as_bytes())
			.unwrap();
	let targets = tg.get(&mut try_catch_scope, targets.into()).unwrap();
	let targets = v8::Local::<v8::Object>::try_from(targets).unwrap();

	// Get the target function.
	let (package, path) = match &module {
		Module::Normal(module) => (module.package, module.module_path.to_string()),
		_ => unreachable!(),
	};
	let key = Key {
		package,
		path,
		name: self.name.clone(),
	};
	let key = serde_json::to_string(&key).unwrap();
	let key = serde_v8::to_v8(&mut try_catch_scope, key).map_err(Error::other)?;
	let function = targets
		.get(&mut try_catch_scope, key)
		.wrap_err("Failed to get the target function.")?;
	let function = v8::Local::<v8::Function>::try_from(function)
		.map_err(Error::other)
		.wrap_err("Expected a function.")?;

	// Move the env to v8.
	let env = self
		.env
		.to_v8(&mut try_catch_scope)
		.wrap_err("Failed to move the env to v8.")?;

	// Set the env.
	let env_object =
		v8::String::new_external_onebyte_static(&mut try_catch_scope, "env".as_bytes()).unwrap();
	let env_object = tg.get(&mut try_catch_scope, env_object.into()).unwrap();
	let env_object = v8::Local::<v8::Object>::try_from(env_object).unwrap();
	let value =
		v8::String::new_external_onebyte_static(&mut try_catch_scope, "value".as_bytes()).unwrap();
	env_object.set(&mut try_catch_scope, value.into(), env);

	// Move the args to v8.
	let args = self
		.args
		.iter()
		.map(|arg| arg.to_v8(&mut try_catch_scope))
		.collect::<Result<Vec<_>>>()
		.wrap_err("Failed to move the args to v8.")?;

	// Call the function.
	let output = function.call(&mut try_catch_scope, undefined.into(), &args);
	let Some(output) = output else {
		let exception = try_catch_scope.exception().unwrap();
		let error = error_from_exception(&mut try_catch_scope, &state, exception);
		return Err(error);
	};

	// Make the output and context global.
	let output = v8::Global::new(&mut try_catch_scope, output);
	let context = v8::Global::new(&mut try_catch_scope, context);

	// Exit the context.
	drop(try_catch_scope);
	drop(context_scope);
	drop(handle_scope);
	drop(isolate);

	// Await the output.
	let output = super::context::await_value(context.clone(), output).await?;

	// Enter the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Local::new(&mut handle_scope, context.clone());
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Move the output to the context.
	let output = v8::Local::new(&mut context_scope, output);

	// Move the output from v8.
	let output = from_v8(&mut context_scope, output)?;

	// Exit the context.
	drop(context_scope);
	drop(handle_scope);
	drop(isolate);

	Ok(output)
}

pub extern "C" fn host_initialize_import_meta_object_callback(
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
		.modules
		.borrow()
		.iter()
		.find(|module| module.v8_identity_hash == identity_hash)
		.unwrap()
		.module
		.clone();
	let module = serde_v8::to_v8(&mut scope, module).unwrap();

	// Set import.meta.module.
	let module_string =
		v8::String::new_external_onebyte_static(&mut scope, "module".as_bytes()).unwrap();
	meta.set(&mut scope, module_string.into(), module).unwrap();
}
pub fn create_context(tg: Server) -> v8::Global<v8::Context> {
	// Create the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Context::new(&mut handle_scope);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Set the server on the context.
	context.set_slot(&mut context_scope, tg);

	// Create the state.
	let state = Rc::new(State {
		global_source_map: Some(SourceMap::from_slice(SOURCE_MAP).unwrap()),
		modules: Rc::new(RefCell::new(Vec::new())),
		futures: Rc::new(RefCell::new(FuturesUnordered::new())),
	});

	// Set the state on the context.
	context.set_slot(&mut context_scope, state);

	// Create the syscall function.
	let syscall_string =
		v8::String::new_external_onebyte_static(&mut context_scope, "syscall".as_bytes()).unwrap();
	let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
	let global = context.global(&mut context_scope);
	global
		.set(&mut context_scope, syscall_string.into(), syscall.into())
		.unwrap();

	// Drop the context scope.
	drop(context_scope);

	v8::Global::new(&mut handle_scope, context)
}

pub async fn await_value(
	context: v8::Global<v8::Context>,
	value: v8::Global<v8::Value>,
) -> Result<v8::Global<v8::Value>> {
	poll_fn(move |cx| await_value_inner(context.clone(), value.clone(), cx)).await
}

pub fn await_value_inner(
	context: v8::Global<v8::Context>,
	value: v8::Global<v8::Value>,
	cx: &mut std::task::Context<'_>,
) -> Poll<Result<v8::Global<v8::Value>>> {
	// Get the state.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Local::new(&mut handle_scope, context);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
	let state = context
		.get_slot::<Rc<State>>(&mut context_scope)
		.unwrap()
		.clone();
	drop(context_scope);
	let context = v8::Global::new(&mut handle_scope, context);
	drop(handle_scope);
	drop(isolate);

	// Poll the context's futures and resolve or reject all that are ready.
	loop {
		// Poll the context's futures.
		let output = match state.futures.borrow_mut().poll_next_unpin(cx) {
			Poll::Ready(Some(output)) => output,
			Poll::Ready(None) => break,
			Poll::Pending => return Poll::Pending,
		};
		let FutureOutput {
			context,
			promise_resolver,
			result,
		} = output;

		// Get the thread local isolate and enter the context.
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Resolve or reject the promise.
		let promise_resolver = v8::Local::new(&mut context_scope, promise_resolver);
		match result {
			Ok(value) => {
				// Resolve the promise.
				let value = v8::Local::new(&mut context_scope, value);
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
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
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

#[allow(clippy::await_holding_refcell_ref)]
pub async fn evaluate_module(
	context: v8::Global<v8::Context>,
	module: &Module,
) -> Result<(v8::Global<v8::Module>, v8::Global<v8::Value>)> {
	// Enter the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Local::new(&mut handle_scope, context.clone());
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Get the state.
	let state = context
		.get_slot::<Rc<State>>(&mut context_scope)
		.unwrap()
		.clone();

	// Load the module.
	let module = load_module(&mut context_scope, module)?;

	// Instantiate the module.
	let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
	let output = module.instantiate_module(&mut try_catch_scope, resolve_module_callback);
	if output.is_none() {
		let exception = try_catch_scope.exception().unwrap();
		let error = error_from_exception(&mut try_catch_scope, &state, exception);
		return Err(error);
	}
	drop(try_catch_scope);

	// Evaluate the module.
	let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
	let module_output = module.evaluate(&mut try_catch_scope);
	let Some(module_output) = module_output else {
		let exception = try_catch_scope.exception().unwrap();
		let error = error_from_exception(&mut try_catch_scope, &state, exception);
		return Err(error);
	};
	drop(try_catch_scope);

	let context = v8::Global::new(&mut context_scope, context);
	let module = v8::Global::new(&mut context_scope, module);
	let output = v8::Global::new(&mut context_scope, module_output);

	// Exit the context.
	drop(context_scope);
	drop(handle_scope);
	drop(isolate);

	// Await the module output.
	let output = await_value(context.clone(), output)
		.await
		.wrap_err("Failed to evaluate the module.")?;

	// Return the module.
	Ok((module, output))
}

/// Load a module.
fn load_module<'s>(
	scope: &mut v8::HandleScope<'s>,
	module: &module::Module,
) -> Result<v8::Local<'s, v8::Module>> {
	// Get the context.
	let context = scope.get_current_context();

	// Get the server.
	let tg = context.get_slot::<Server>(scope).unwrap().clone();

	// Get the state.
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();

	// Return a cached module if this module has already been loaded.
	if let Some(module) = state
		.modules
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
	let script_id = state.modules.borrow().len().to_i32().unwrap() + 1;
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
	tg.main_runtime_handle.spawn({
		let tg = tg.clone();
		let module = module.clone();
		async move {
			// Load the module.
			let text = match module.load(&tg).await {
				Ok(text) => text,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};

			// Transpile the module.
			let Module::Normal(_) = module else {
				return sender
					.send(Err(Error::message("The module must be a normal module.")))
					.unwrap();
			};
			let output = match Module::transpile(text.clone()) {
				Ok(transpile_output) => transpile_output,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};

			// Send the output.
			let output = (text, output.transpiled_text, output.source_map);
			sender.send(Ok(output)).unwrap();
		}
	});
	let (text, transpiled_text, source_map) = receiver
		.recv()
		.unwrap()
		.wrap_err_with(|| format!(r#"Failed to load module "{module}"."#))?;
	let source_map = SourceMap::from_slice(source_map.as_bytes())
		.map_err(Error::other)
		.wrap_err("Failed to parse the source map.")?;

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
	state.modules.borrow_mut().push(state::Module {
		v8_identity_hash: v8_module.get_identity_hash(),
		v8_module: v8::Global::new(scope, v8_module),
		module: module.clone(),
		text,
		transpiled_text: Some(transpiled_text),
		source_map: Some(source_map),
	});

	Ok(v8_module)
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

fn resolve_module_callback_inner<'s>(
	context: v8::Local<'s, v8::Context>,
	specifier: v8::Local<'s, v8::String>,
	_import_assertions: v8::Local<'s, v8::FixedArray>,
	referrer: v8::Local<'s, v8::Module>,
) -> Result<v8::Local<'s, v8::Module>> {
	// Get a scope for the callback.
	let mut scope = unsafe { v8::CallbackScope::new(context) };

	// Get the server.
	let tg = context.get_slot::<Server>(&mut scope).unwrap().clone();

	// Get the state.
	let state = context.get_slot::<Rc<State>>(&mut scope).unwrap().clone();

	// Parse the specifier.
	let specifier = specifier.to_rust_string_lossy(&mut scope);
	let import: module::Import = specifier.parse()?;

	// Get the referrer.
	let referrer_identity_hash = referrer.get_identity_hash();
	let module = state
		.modules
		.borrow()
		.iter()
		.find(|module| module.v8_identity_hash == referrer_identity_hash)
		.wrap_err_with(|| {
			format!(
				r#"Unable to find the referrer module with identity hash "{referrer_identity_hash}"."#
			)
		})?
		.module
		.clone();

	// Resolve.
	let (sender, receiver) = std::sync::mpsc::channel();
	tg.main_runtime_handle.spawn({
		let tg = tg.clone();
		let import = import.clone();
		let module = module.clone();
		async move {
			let module = module.resolve(&tg, &import).await;
			sender.send(module).unwrap();
		}
	});
	let module = receiver
		.recv()
		.unwrap()
		.wrap_err_with(|| format!(r#"Failed to resolve "{import}" relative to "{module}"."#))?;

	// Load.
	let module = load_module(&mut scope, &module).wrap_err(r#"Failed to load the module."#)?;

	Ok(module)
}

#[allow(clippy::too_many_lines)]
pub fn error_from_exception(
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

	// If the exception is not a native error, then attempt to deserialize it as a Tangram Error.
	if !exception.is_native_error() {
		if let Ok(error) = serde_v8::from_v8(scope, exception) {
			return error;
		}
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

	// Create the error.
	Error::Build(build::Error::Target(super::Error {
		message,
		location,
		stack_trace,
		source,
	}))
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
		let modules = state.modules.borrow();
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
