use self::{
	convert::{from_v8, ToV8},
	syscall::syscall,
};
use crate::{
	language::{self, Import, Module},
	run, Client, Package, Result, Task, Value, WrapErr,
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
	run_state: Arc<run::State>,
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

pub async fn run(
	client: Client,
	task: Task,
	state: Arc<run::State>,
	main_runtime_handle: tokio::runtime::Handle,
) -> Option<Value> {
	// Create the isolate params.
	let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);

	// Create the isolate.
	let mut isolate = v8::Isolate::new(params);

	// Set the host import module dynamically callback.
	isolate.set_host_import_module_dynamically_callback(host_import_module_dynamically_callback);

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
		run_state: state,
		global_source_map: Some(SourceMap::from_slice(SOURCE_MAP).unwrap()),
		loaded_modules: Rc::new(RefCell::new(Vec::new())),
		futures: Rc::new(RefCell::new(FuturesUnordered::new())),
	});

	// Set the state on the context.
	context.set_slot(&mut context_scope, state.clone());

	// Create the syscall function.
	let syscall_string =
		v8::String::new_external_onebyte_static(&mut context_scope, "syscall".as_bytes()).unwrap();
	let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
	let global = context.global(&mut context_scope);
	global
		.set(&mut context_scope, syscall_string.into(), syscall.into())
		.unwrap();

	// Get the tg global.
	let global = context.global(&mut context_scope);
	let tg = v8::String::new_external_onebyte_static(&mut context_scope, "tg".as_bytes()).unwrap();
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
	let output = poll_fn(move |cx| {
		// Poll the context's futures and resolve or reject all that are ready.
		loop {
			// Poll the context's futures.
			let (result, promise_resolver) = match state.futures.borrow_mut().poll_next_unpin(cx) {
				Poll::Ready(Some(output)) => output,
				Poll::Ready(None) => break,
				Poll::Pending => return Poll::Pending,
			};

			// Enter the context.
			let mut handle_scope = v8::HandleScope::new(&mut isolate);
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
		let mut handle_scope = v8::HandleScope::new(&mut isolate);
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Handle the value.
		let output = v8::Local::new(&mut context_scope, output.clone());
		let output = match v8::Local::<v8::Promise>::try_from(output) {
			Err(_) => output,
			Ok(promise) => match promise.state() {
				v8::PromiseState::Pending => return Poll::Pending,
				v8::PromiseState::Fulfilled => promise.result(&mut context_scope),
				v8::PromiseState::Rejected => {
					let exception = promise.result(&mut context_scope);
					let error = Error::with_exception(&mut context_scope, &state, exception);
					return Poll::Ready(Err(error));
				},
			},
		};

		// Move the output from V8.
		let output = match from_v8(&mut context_scope, output) {
			Ok(output) => output,
			Err(error) => return Poll::Ready(Err(todo!())),
		};

		Poll::Ready(Ok(output))
	})
	.await;

	let output = match output {
		Ok(output) => output,
		Err(error) => {
			return None;
		},
	};

	Some(output)
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
		let error = Error::with_exception(&mut try_catch_scope, &state, exception);
		return Err(todo!());
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
