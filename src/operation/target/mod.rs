use self::syscall::syscall;
use super::Target;
use crate::{
	compiler::{self, Compiler},
	value::Value,
	Cli, State,
};
use anyhow::{anyhow, bail, Context, Result};
use futures::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::{cell::RefCell, fmt::Write, future::poll_fn, num::NonZeroI32, rc::Rc, task::Poll};

mod syscall;

impl State {
	pub(super) async fn run_target(&self, target: &Target) -> Result<Value> {
		// Run the target on the local pool because it is a `!Send` future because it uses v8.
		let output = self
			.local_pool_handle
			.spawn_pinned({
				let main_runtime_handle = tokio::runtime::Handle::current();
				let cli = self.upgrade();
				let target = target.clone();
				move || async move { run_target_inner(cli, main_runtime_handle, &target).await }
			})
			.await
			.context("Failed to join the task.")?
			.context("Failed to run the target.")?;

		Ok(output)
	}
}

thread_local! {
	static THREAD_LOCAL_ISOLATE: Rc<RefCell<v8::OwnedIsolate>> = {
		// Create the isolate.
		let params = v8::CreateParams::default();
		let isolate = Rc::new(RefCell::new(v8::Isolate::new(params)));

		// Configure the isolate.
		isolate.borrow_mut().set_capture_stack_trace_for_uncaught_exceptions(true, 10);

		isolate
	};
}

struct ContextState {
	cli: Cli,
	compiler: Compiler,
	main_runtime_handle: tokio::runtime::Handle,
	modules: Rc<RefCell<Vec<Module>>>,
	futures: Rc<RefCell<FuturesUnordered<LocalBoxFuture<'static, FutureOutput>>>>,
}

#[derive(Debug)]
struct Module {
	identity_hash: NonZeroI32,
	module: v8::Global<v8::Module>,
	url: compiler::Url,
	source: String,
	_transpiled: Option<String>,
	source_map: Option<SourceMap>,
}

struct FutureOutput {
	context: v8::Global<v8::Context>,
	promise_resolver: v8::Global<v8::PromiseResolver>,
	result: Result<v8::Global<v8::Value>>,
}

#[allow(clippy::too_many_lines)]
async fn run_target_inner(
	cli: Cli,
	main_runtime_handle: tokio::runtime::Handle,
	target: &Target,
) -> Result<Value> {
	// Lock the cli.
	let state = cli.lock_shared().await?;

	// Retrieve the thread local isolate.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);

	// Create the context and context state.
	let (context, context_state) = {
		// Create the context.
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Context::new(&mut handle_scope);

		// Create the context state.
		let context_state = Rc::new(ContextState {
			cli: cli.clone(),
			compiler: Compiler::new(cli.clone()),
			main_runtime_handle: main_runtime_handle.clone(),
			modules: Rc::new(RefCell::new(Vec::new())),
			futures: Rc::new(RefCell::new(FuturesUnordered::new())),
		});

		// Set the context state on the context.
		context.set_slot(&mut handle_scope, Rc::clone(&context_state));

		// Create a context scope.
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Create the global syscall function.
		let syscall_string = v8::String::new(&mut context_scope, "syscall").unwrap();
		let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
		context
			.global(&mut context_scope)
			.set(&mut context_scope, syscall_string.into(), syscall.into())
			.unwrap();

		// // Create the tg global.
		// let module = load_module(
		// 	&mut context_scope,
		// 	&compiler::Url::new_core("/mod.ts".into()),
		// )
		// .context("Failed to load the core module.")?;
		// evaluate_module(&mut context_scope, module)
		// 	.await
		// 	.context("Failed to evaluate the core module.")?;
		// let tg = module.get_module_namespace();
		// let tg_string = v8::String::new(&mut context_scope, "tg").unwrap();
		// context
		// 	.global(&mut context_scope)
		// 	.set(&mut context_scope, tg_string.into(), tg)
		// 	.unwrap();

		drop(context_scope);

		// Make the context global.
		let context = v8::Global::new(&mut handle_scope, context);

		(context, context_state)
	};

	// Get the package's entrypoint path.
	let entrypoint_path = state
		.get_package_entrypoint_path(target.package)
		.context("Failed to retrieve the package entrypoint.")?
		.context("The package must have an entrypoint.")?;

	// Create the module url.
	let url = compiler::Url::new_hash(target.package, entrypoint_path);

	// Evaluate the module.
	let (module, module_output) = {
		// Enter the context.
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Load the module.
		let module = load_module(&mut context_scope, &url)?;

		// Instantiate the module.
		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
		let output = module.instantiate_module(&mut try_catch_scope, resolve_module_callback);
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let exception = exception_to_string(&mut scope, &context_state, exception);
			bail!("{exception}");
		}
		output.unwrap();
		drop(try_catch_scope);

		// Evaluate the module.
		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
		let module_output = module.evaluate(&mut try_catch_scope);
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let exception = exception_to_string(&mut try_catch_scope, &context_state, exception);
			bail!("{exception}");
		}
		let module_output = module_output.unwrap();
		drop(try_catch_scope);

		(
			v8::Global::new(&mut context_scope, module),
			v8::Global::new(&mut context_scope, module_output),
		)
	};

	// Await the module output.
	await_value(context.clone(), Rc::clone(&context_state), module_output)
		.await
		.context("Failed to evaluate the module.")?;

	let output = {
		// Enter the context.
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Move the module to the context.
		let module = v8::Local::new(&mut context_scope, module);

		// Get the module namespace.
		let namespace = module.get_module_namespace();
		let namespace = namespace.to_object(&mut context_scope).unwrap();

		// Get the target export.
		let target_name_string = v8::String::new(&mut context_scope, &target.name).unwrap();
		let target_export = namespace
			.get(&mut context_scope, target_name_string.into())
			.context("Failed to get the target export.")?;

		// Get the target export run function.
		let target_export = <v8::Local<v8::Object>>::try_from(target_export)
			.context("The target export must be an object.")?;
		let run_string = v8::String::new(&mut context_scope, "run").unwrap();
		let target_export_run_function =
			target_export
				.get(&mut context_scope, run_string.into())
				.context(r#"The target export must contain the key "run"."#)?;
		let target_export_run_function =
			<v8::Local<v8::Function>>::try_from(target_export_run_function)
				.context(r#"The value for the target export key "run" must be a function."#)?;

		// Serialize the args to v8.
		let args = target
			.args
			.iter()
			.map(|arg| {
				let arg = serde_v8::to_v8(&mut context_scope, arg)?;
				Ok(arg)
			})
			.collect::<Result<Vec<_>>>()?;

		// Call the target export run function.
		let undefined = v8::undefined(&mut context_scope);
		let output = target_export_run_function
			.call(&mut context_scope, undefined.into(), &args)
			.unwrap();

		v8::Global::new(&mut context_scope, output)
	};

	// Await the output.
	let output = await_value(context.clone(), Rc::clone(&context_state), output).await?;

	// Get the output and deserialize it from v8.
	let output = {
		// Enter the context.
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Move the output to the context.
		let output = v8::Local::new(&mut context_scope, output);

		// Deserialize the output from v8.
		serde_v8::from_v8(&mut context_scope, output)?
	};

	Ok(output)
}

async fn await_value(
	context: v8::Global<v8::Context>,
	context_state: Rc<ContextState>,
	value: v8::Global<v8::Value>,
) -> Result<v8::Global<v8::Value>> {
	let context = context.clone();
	let value = poll_fn(move |cx| {
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);

		// Poll the futures and handle all that are ready.
		loop {
			// Poll the outstanding futures.
			let output = match context_state.futures.borrow_mut().poll_next_unpin(cx) {
				Poll::Ready(Some(output)) => output,
				Poll::Ready(None) => break,
				Poll::Pending => return Poll::Pending,
			};
			let FutureOutput {
				context,
				promise_resolver,
				result,
			} = output;

			// Retrieve the thread local isolate and enter the context.
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
					let error = v8::String::new(&mut context_scope, &error.to_string()).unwrap();
					let error = v8::Local::new(&mut context_scope, error);
					promise_resolver.reject(&mut context_scope, error.into());
				},
			};
		}

		// Enter the context.
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, &context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Handle the value.
		let value = v8::Local::new(&mut context_scope, value.clone());
		match v8::Local::<v8::Promise>::try_from(value) {
			Err(_) => {
				let value = v8::Global::new(&mut context_scope, value);
				Poll::Ready(Ok::<_, anyhow::Error>(value))
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
					let exception =
						exception_to_string(&mut context_scope, &context_state, exception);
					Poll::Ready(Err(anyhow!("{exception}")))
				},
			},
		}
	})
	.await?;
	Ok(value)
}

// async fn evaluate_module<'s>(
// 	scope: &mut v8::HandleScope<'s>,
// 	module: v8::Local<'s, v8::Module>,
// ) -> Result<v8::Local<'s, v8::Value>> {
// 	todo!();
// }

/// Load a module at the specified URL.
fn load_module<'s>(
	scope: &mut v8::HandleScope<'s>,
	url: &compiler::Url,
) -> Result<v8::Local<'s, v8::Module>> {
	// Get the context and context scope.
	let context = scope.get_current_context();
	let context_state = Rc::clone(context.get_slot::<Rc<ContextState>>(scope).unwrap());

	// Return a cached module if this URL has already been loaded.
	if let Some(module) = context_state
		.modules
		.borrow()
		.iter()
		.find(|module| &module.url == url)
	{
		let module = v8::Local::new(scope, &module.module);
		return Ok(module);
	}

	// Define the module's origin.
	let resource_name = v8::String::new(scope, &url.to_string()).unwrap();
	let resource_line_offset = 0;
	let resource_column_offset = 0;
	let resource_is_shared_cross_origin = false;
	let script_id = context_state.modules.borrow().len().to_i32().unwrap() + 1;
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
	context_state.main_runtime_handle.spawn({
		let compiler = context_state.compiler.clone();
		let url = url.clone();
		async move {
			// Load the module.
			let source = match compiler.load(&url).await {
				Ok(source) => source,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};

			// Transpile the module.
			let transpile_output = match compiler.transpile(source.clone()).await {
				Ok(transpile_output) => transpile_output,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};

			// Send the output.
			let output = (
				source,
				transpile_output.transpiled,
				transpile_output.source_map,
			);
			sender.send(Ok(output)).unwrap();
		}
	});
	let (source, transpiled, source_map) = receiver
		.recv()
		.unwrap()
		.with_context(|| format!(r#"Failed to load from URL "{url}"."#))?;
	let source_map =
		SourceMap::from_slice(source_map.as_bytes()).context("Failed to parse the source map.")?;

	// Compile the module.
	let mut try_catch_scope = v8::TryCatch::new(scope);
	let module_source = v8::String::new(&mut try_catch_scope, &transpiled).unwrap();
	let module_source = v8::script_compiler::Source::new(module_source, Some(&origin));
	let module = v8::script_compiler::compile_module(&mut try_catch_scope, module_source);
	if try_catch_scope.has_caught() {
		let exception = try_catch_scope.exception().unwrap();
		let mut scope = v8::HandleScope::new(&mut try_catch_scope);
		let exception = exception_to_string(&mut scope, &context_state, exception);
		bail!("{exception}");
	}
	let module = module.unwrap();
	drop(try_catch_scope);

	// Cache the module.
	context_state.modules.borrow_mut().push(Module {
		identity_hash: module.get_identity_hash(),
		module: v8::Global::new(scope, module),
		url: url.clone(),
		source,
		_transpiled: Some(transpiled),
		source_map: Some(source_map),
	});

	Ok(module)
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
			let error = v8::String::new(&mut scope, &error.to_string()).unwrap();
			scope.throw_exception(error.into());
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
	// Get a scope for the callback and the context state.
	let mut scope = unsafe { v8::CallbackScope::new(context) };
	let context_state = Rc::clone(context.get_slot::<Rc<ContextState>>(&mut scope).unwrap());

	// Get the specifier.
	let specifier = specifier.to_rust_string_lossy(&mut scope);

	// Get the referrer URL.
	let referrer_identity_hash = referrer.get_identity_hash();
	let referrer = context_state
		.modules
		.borrow()
		.iter()
		.find(|module| module.identity_hash == referrer_identity_hash)
		.with_context(|| {
			format!(
				r#"Unable to find the referrer module with identity hash "{referrer_identity_hash}"."#
			)
		})?
		.url
		.clone();

	// Resolve.
	let (sender, receiver) = std::sync::mpsc::channel();
	context_state.main_runtime_handle.spawn({
		let compiler = context_state.compiler.clone();
		let specifier = specifier.clone();
		let referrer = referrer.clone();
		async move {
			let url = compiler.resolve(&specifier, Some(&referrer)).await;
			sender.send(url).unwrap();
		}
	});
	let url = receiver.recv().unwrap().with_context(|| {
		format!(r#"Failed to resolve specifier "{specifier}" relative to referrer "{referrer:?}"."#)
	})?;

	// Load.
	let module = load_module(&mut scope, &url)
		.with_context(|| format!(r#"Failed to load the module with URL "{url}"."#))?;

	Ok(module)
}

/// Render an exception to a string. The string will include the exception's message and a stack trace.
fn exception_to_string(
	scope: &mut v8::HandleScope,
	context_state: &ContextState,
	exception: v8::Local<v8::Value>,
) -> String {
	let mut string = String::new();

	// Write the exception message.
	let message = exception
		.to_string(scope)
		.unwrap()
		.to_rust_string_lossy(scope);
	writeln!(string, "{message}").unwrap();

	// Write the stack trace if one is available.
	if let Some(stack_trace) = v8::Exception::get_stack_trace(scope, exception) {
		// Write the stack trace one frame at a time.
		for i in 0..stack_trace.get_frame_count() {
			// Retrieve the URL, line, and column.
			let stack_trace_frame = stack_trace.get_frame(scope, i).unwrap();
			let url = stack_trace_frame
				.get_script_name(scope)
				.unwrap()
				.to_rust_string_lossy(scope)
				.parse()
				.unwrap();
			let line = stack_trace_frame.get_line_number().to_u32().unwrap() - 1;
			let column = stack_trace_frame.get_column().to_u32().unwrap() - 1;

			// Apply a source map if one is available.
			let (line, column) = context_state
				.modules
				.borrow()
				.iter()
				.find(|module| module.url == url)
				.and_then(|module| module.source_map.as_ref())
				.map_or((line, column), |source_map| {
					let token = source_map.lookup_token(line, column).unwrap();
					let line = token.get_src_line();
					let column = token.get_src_col();
					(line, column)
				});

			// Write the URL, line, and column.
			write!(string, "{url}:{}:{}", line + 1, column + 1).unwrap();

			// Add a newline if this is not the last frame.
			if i < stack_trace.get_frame_count() - 1 {
				writeln!(string).unwrap();
			}
		}
	}

	string
}
