use self::{
	convert::{from_v8, ToV8},
	syscall::syscall,
};
use super::Progress;
use futures::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::{cell::RefCell, future::poll_fn, num::NonZeroI32, rc::Rc, str::FromStr, task::Poll};
use tangram_client::{
	module::{self, Import, Module},
	Client, Result, Target, WrapErr,
};

mod convert;
mod error;
mod syscall;

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/runtime.heapsnapshot"));

const SOURCE_MAP: &[u8] = include_bytes!(concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/src/js/runtime.js.map"
));

struct State {
	client: Box<dyn Client>,
	futures: Rc<RefCell<Futures>>,
	global_source_map: Option<SourceMap>,
	loaded_modules: Rc<RefCell<Vec<LoadedModule>>>,
	main_runtime_handle: tokio::runtime::Handle,
	progress: Box<dyn Progress>,
}

type Futures = FuturesUnordered<
	LocalBoxFuture<'static, (Result<Box<dyn ToV8>>, v8::Global<v8::PromiseResolver>)>,
>;

struct LoadedModule {
	module: Module,
	source_map: Option<SourceMap>,
	v8_identity_hash: NonZeroI32,
	v8_module: v8::Global<v8::Module>,
}

#[allow(clippy::too_many_lines)]
pub async fn run(
	client: &dyn Client,
	target: Target,
	progress: &dyn Progress,
	main_runtime_handle: tokio::runtime::Handle,
) {
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

	// Create the state.
	let state = Rc::new(State {
		client: client.clone_box(),
		futures: Rc::new(RefCell::new(FuturesUnordered::new())),
		global_source_map: Some(SourceMap::from_slice(SOURCE_MAP).unwrap()),
		loaded_modules: Rc::new(RefCell::new(Vec::new())),
		main_runtime_handle,
		progress: progress.clone_box(),
	});

	// Create the context.
	let context = {
		// Create and enter the context.
		let scope = &mut v8::HandleScope::new(&mut isolate);
		let context = v8::Context::new(scope);
		let scope = &mut v8::ContextScope::new(scope, context);

		// Set the state on the context.
		context.set_slot(scope, state.clone());

		// Create the syscall function.
		let syscall_string =
			v8::String::new_external_onebyte_static(scope, "syscall".as_bytes()).unwrap();
		let syscall = v8::Function::new(scope, syscall).unwrap();
		let global = context.global(scope);
		global
			.set(scope, syscall_string.into(), syscall.into())
			.unwrap();

		v8::Global::new(scope, context)
	};

	let output = {
		// Enter the context.
		let scope = &mut v8::HandleScope::new(&mut isolate);
		let context = v8::Local::new(scope, context.clone());
		let scope = &mut v8::ContextScope::new(scope, context);

		// Get the tg global.
		let global = context.global(scope);
		let tg = v8::String::new_external_onebyte_static(scope, "tg".as_bytes()).unwrap();
		let tg = global.get(scope, tg.into()).unwrap();
		let tg = v8::Local::<v8::Object>::try_from(tg).unwrap();

		// Get the main function.
		let main = v8::String::new_external_onebyte_static(scope, "main".as_bytes()).unwrap();
		let main = tg.get(scope, main.into()).unwrap();
		let main = v8::Local::<v8::Function>::try_from(main).unwrap();

		// Call the main function.
		let undefined = v8::undefined(scope);
		let target = target.to_v8(scope).unwrap();
		let output = main.call(scope, undefined.into(), &[target]).unwrap();

		v8::Global::new(scope, output)
	};

	// Await the output.
	let output = poll_fn(|cx| {
		// Poll the context's futures and resolve or reject all that are ready.
		loop {
			// Poll the context's futures.
			let (result, promise_resolver) = match state.futures.borrow_mut().poll_next_unpin(cx) {
				Poll::Ready(Some(output)) => output,
				Poll::Ready(None) => break,
				Poll::Pending => return Poll::Pending,
			};

			// Enter the context.
			let scope = &mut v8::HandleScope::new(&mut isolate);
			let context = v8::Local::new(scope, context.clone());
			let scope = &mut v8::ContextScope::new(scope, context);

			// Resolve or reject the promise.
			let promise_resolver = v8::Local::new(scope, promise_resolver);
			match result.and_then(|value| value.to_v8(scope)) {
				Ok(value) => {
					// Resolve the promise.
					promise_resolver.resolve(scope, value);
				},
				Err(error) => {
					// Reject the promise.
					let exception = error::to_exception(scope, &error);
					promise_resolver.reject(scope, exception);
				},
			};
		}

		// Enter the context.
		let scope = &mut v8::HandleScope::new(&mut isolate);
		let context = v8::Local::new(scope, context.clone());
		let scope = &mut v8::ContextScope::new(scope, context);

		// Handle the value.
		let output = v8::Local::new(scope, output.clone());
		let output = match v8::Local::<v8::Promise>::try_from(output) {
			Err(_) => output,
			Ok(promise) => match promise.state() {
				v8::PromiseState::Pending => return Poll::Pending,
				v8::PromiseState::Fulfilled => promise.result(scope),
				v8::PromiseState::Rejected => {
					let exception = promise.result(scope);
					let state = state.clone();
					let error = self::error::from_exception(&state, scope, exception);
					return Poll::Ready(Err(error));
				},
			},
		};

		// Move the output from V8.
		let output = match from_v8(scope, output) {
			Ok(output) => output,
			Err(error) => {
				return Poll::Ready(Err(error));
			},
		};

		Poll::Ready(Ok(output))
	})
	.await;

	match output {
		Ok(output) => {
			state.progress.output(Some(output));
		},
		Err(error) => {
			let trace = error.trace().to_string();
			state.progress.log(trace.into_bytes());
			state.progress.output(None);
		},
	};
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
	let module = if resource_name == "[runtime]" {
		let module = specifier.to_rust_string_lossy(scope);
		match Module::from_str(&module) {
			Ok(module) => module,
			Err(error) => {
				let exception = error::to_exception(scope, &error);
				scope.throw_exception(exception);
				return None;
			},
		}
	} else {
		// Get the module.
		let module = match Module::from_str(&resource_name) {
			Ok(module) => module,
			Err(error) => {
				let exception = error::to_exception(scope, &error);
				scope.throw_exception(exception);
				return None;
			},
		};

		// Get the import.
		let import = specifier.to_rust_string_lossy(scope);
		let import = match Import::from_str(&import) {
			Ok(import) => import,
			Err(error) => {
				let exception = error::to_exception(scope, &error);
				scope.throw_exception(exception);
				return None;
			},
		};

		match resolve_module(scope, &module, &import) {
			Some(module) => module,
			None => {
				return None;
			},
		}
	};

	// Load the module.
	let Some(module) = load_module(scope, &module) else {
		return None;
	};

	// Instantiate the module.
	module.instantiate_module(scope, resolve_module_callback)?;

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
	_import_assertions: v8::Local<'s, v8::FixedArray>,
	referrer: v8::Local<'s, v8::Module>,
) -> Option<v8::Local<'s, v8::Module>> {
	// Get a scope for the callback.
	let scope = unsafe { &mut v8::CallbackScope::new(context) };

	// Get the state.
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();

	// Get the module.
	let identity_hash = referrer.get_identity_hash();
	let module = match state
		.loaded_modules
		.borrow()
		.iter()
		.find(|loaded_module| loaded_module.v8_identity_hash == identity_hash)
		.map(|loaded_module| loaded_module.module.clone())
		.wrap_err_with(|| {
			format!(r#"Unable to find the module with identity hash "{identity_hash}"."#)
		}) {
		Ok(loaded_module) => loaded_module,
		Err(error) => {
			let exception = error::to_exception(scope, &error);
			scope.throw_exception(exception);
			return None;
		},
	};

	// Get the import.
	let specifier = specifier.to_rust_string_lossy(scope);
	let import = match Import::from_str(&specifier).wrap_err("Failed to parse the import.") {
		Ok(import) => import,
		Err(error) => {
			let exception = error::to_exception(scope, &error);
			scope.throw_exception(exception);
			return None;
		},
	};

	// Resolve the module.
	let Some(module) = resolve_module(scope, &module, &import) else {
		return None;
	};

	// Load the module.
	let Some(module) = load_module(scope, &module) else {
		return None;
	};

	Some(module)
}

/// Resolve a module.
fn resolve_module(scope: &mut v8::HandleScope, module: &Module, import: &Import) -> Option<Module> {
	let context = scope.get_current_context();
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();
	let (sender, receiver) = std::sync::mpsc::channel();
	state.main_runtime_handle.spawn({
		let client = state.client.clone_box();
		let module = module.clone();
		let import = import.clone();
		async move {
			let module = module.resolve(client.as_ref(), None, &import).await;
			sender.send(module).unwrap();
		}
	});
	let module = match receiver
		.recv()
		.unwrap()
		.wrap_err_with(|| format!(r#"Failed to resolve "{import}" relative to "{module}"."#))
	{
		Ok(module) => module,
		Err(error) => {
			let exception = error::to_exception(scope, &error);
			scope.throw_exception(exception);
			return None;
		},
	};
	Some(module)
}

/// Load a module.
fn load_module<'s>(
	scope: &mut v8::HandleScope<'s>,
	module: &Module,
) -> Option<v8::Local<'s, v8::Module>> {
	// Get the context and state.
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
		return Some(module);
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
		let client = state.client.clone_box();
		let module = module.clone();
		async move {
			let result = module.load(client.as_ref(), None).await;
			sender.send(result).unwrap();
		}
	});
	let text = match receiver
		.recv()
		.unwrap()
		.wrap_err_with(|| format!(r#"Failed to load module "{module}"."#))
	{
		Ok(text) => text,
		Err(error) => {
			let exception = error::to_exception(scope, &error);
			scope.throw_exception(exception);
			return None;
		},
	};

	// Transpile the module.
	let module::transpile::Output {
		transpiled_text,
		source_map,
	} = match Module::transpile(text).wrap_err("Failed to transpile the module.") {
		Ok(output) => output,
		Err(error) => {
			let exception = error::to_exception(scope, &error);
			scope.throw_exception(exception);
			return None;
		},
	};

	// Parse the source map.
	let source_map = match SourceMap::from_slice(source_map.as_bytes())
		.wrap_err("Failed to parse the source map.")
	{
		Ok(source_map) => source_map,
		Err(error) => {
			let exception = error::to_exception(scope, &error);
			scope.throw_exception(exception);
			return None;
		},
	};

	// Compile the module.
	let source = v8::String::new(scope, &transpiled_text).unwrap();
	let source = v8::script_compiler::Source::new(source, Some(&origin));
	let Some(v8_module) = v8::script_compiler::compile_module(scope, source) else {
		return None;
	};

	// Cache the module.
	state.loaded_modules.borrow_mut().push(LoadedModule {
		module: module.clone(),
		source_map: Some(source_map),
		v8_identity_hash: v8_module.get_identity_hash(),
		v8_module: v8::Global::new(scope, v8_module),
	});

	Some(v8_module)
}

/// Implement V8's import.meta callback.
extern "C" fn host_initialize_import_meta_object_callback(
	context: v8::Local<v8::Context>,
	module: v8::Local<v8::Module>,
	meta: v8::Local<v8::Object>,
) {
	// Create the scope.
	let scope = unsafe { &mut v8::CallbackScope::new(context) };

	// Get the state.
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();

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

	// Set import.meta.url.
	let key = v8::String::new_external_onebyte_static(scope, "url".as_bytes()).unwrap();
	let value = v8::String::new(scope, &module.to_string()).unwrap();
	meta.set(scope, key.into(), value.into()).unwrap();
}
