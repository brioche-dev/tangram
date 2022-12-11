use self::syscall::syscall;
use crate::{
	compiler::{self, Compiler},
	expression::Target,
	hash::Hash,
	{Cli, State},
};
use anyhow::{anyhow, bail, Context, Result};
use futures::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::{cell::RefCell, fmt::Write, future::poll_fn, num::NonZeroI32, rc::Rc, task::Poll};

mod syscall;

impl State {
	pub async fn evaluate_target_js(&self, hash: Hash, target: &Target) -> Result<Hash> {
		// Evaluate the target on the local pool.
		let output_hash = self
			.local_pool_handle
			.spawn_pinned({
				let main_runtime_handle = tokio::runtime::Handle::current();
				let cli = self.upgrade();
				let target = target.clone();
				move || async move { evaluate_target_js(cli, main_runtime_handle, hash, &target).await }
			})
			.await
			.context("Failed to join the task.")?
			.context("Failed to evaluate the target.")?;

		// Evaluate the expression.
		let output_hash = self
			.evaluate(output_hash, hash)
			.await
			.context("Failed to evaluate the expression returned by the JS process.")?;

		Ok(output_hash)
	}
}

// Create the thread local isolate.
thread_local! {
	static THREAD_LOCAL_ISOLATE: Rc<RefCell<v8::OwnedIsolate>> = {
		// Create the isolate.
		let params = v8::CreateParams::default();
		let isolate = Rc::new(RefCell::new(v8::Isolate::new(params)));

		// Configure the isolate.
		isolate.borrow_mut().set_capture_stack_trace_for_uncaught_exceptions(true, 10);

		// Create the isolate state.
		let state = Rc::new(RefCell::new(IsolateState {
			futures: FuturesUnordered::new(),
		}));
		isolate.borrow_mut().set_slot(state);

		isolate
	};
}

struct IsolateState {
	futures: FuturesUnordered<LocalBoxFuture<'static, FutureOutput>>,
}

struct FutureOutput {
	context: v8::Global<v8::Context>,
	promise_resolver: v8::Global<v8::PromiseResolver>,
	result: Result<v8::Global<v8::Value>>,
}

struct ContextState {
	cli: Cli,
	compiler: Compiler,
	main_runtime_handle: tokio::runtime::Handle,
	modules: Vec<Module>,
	name: Option<String>,
	args: Option<Hash>,
	output: Option<Hash>,
}

#[derive(Debug)]
struct Module {
	identity_hash: NonZeroI32,
	module: v8::Global<v8::Module>,
	url: compiler::Url,
	_source: String,
	_transpiled_source: Option<String>,
	source_map: Option<SourceMap>,
}

#[allow(clippy::too_many_lines)]
async fn evaluate_target_js(
	cli: Cli,
	main_runtime_handle: tokio::runtime::Handle,
	hash: Hash,
	target: &Target,
) -> Result<Hash> {
	// Lock the cli.
	let state = cli.lock_shared().await?;

	// Evaluate the package.
	let package_hash = state.evaluate(target.package, hash).await?;

	// Retrieve the thread local isolate.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);

	// Create the context.
	let (context, context_state) = {
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Context::new(&mut handle_scope);
		let context_state = Rc::new(RefCell::new(ContextState {
			cli: cli.clone(),
			compiler: Compiler::new(cli.clone()),
			main_runtime_handle: main_runtime_handle.clone(),
			modules: Vec::new(),
			name: None,
			args: None,
			output: None,
		}));
		context.set_slot(&mut handle_scope, Rc::clone(&context_state));
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
		let tangram = v8::Object::new(&mut context_scope);
		let syscall_string = v8::String::new(&mut context_scope, "syscall").unwrap();
		let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
		tangram
			.set(&mut context_scope, syscall_string.into(), syscall.into())
			.unwrap();
		let type_symbol_string = v8::String::new(&mut context_scope, "typeSymbol").unwrap();
		let type_symbol = v8::Symbol::new(&mut context_scope, None);
		tangram
			.set(
				&mut context_scope,
				type_symbol_string.into(),
				type_symbol.into(),
			)
			.unwrap();
		let tangram_string = v8::String::new(&mut context_scope, "Tangram").unwrap();
		context
			.global(&mut context_scope)
			.set(&mut context_scope, tangram_string.into(), tangram.into())
			.unwrap();
		drop(context_scope);
		let context = v8::Global::new(&mut handle_scope, context);
		(context, context_state)
	};

	// Set the name and args in the state.
	context_state.borrow_mut().name.replace(target.name.clone());
	context_state.borrow_mut().args.replace(target.args);

	// Get the package's entrypoint.
	let entrypoint = state
		.get_package_entrypoint(package_hash)
		.context("Failed to retrieve the package entrypoint.")?
		.context("The package must have an entrypoint.")?;

	// Create the URL.
	let url = compiler::Url::new_hash_target(package_hash, entrypoint);

	let output = {
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
			let exception = exception_to_string(&mut scope, &context_state.borrow(), exception);
			bail!("{exception}");
		}
		output.unwrap();
		drop(try_catch_scope);

		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
		let output = module.evaluate(&mut try_catch_scope);
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let exception =
				exception_to_string(&mut try_catch_scope, &context_state.borrow(), exception);
			bail!("{exception}");
		}
		let output = output.unwrap();
		drop(try_catch_scope);

		// Return the output as a global.
		v8::Global::new(&mut context_scope, output)
	};

	// Poll the isolate's futures until the promise is resolved.
	poll_fn({
		let context = context.clone();
		let context_state = Rc::clone(&context_state);
		move |cx| {
			let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
			let isolate_state = Rc::clone(
				isolate
					.borrow()
					.get_slot::<Rc<RefCell<IsolateState>>>()
					.unwrap(),
			);

			// Poll the futures and handle all that are ready.
			loop {
				// Poll the outstanding futures.
				let output = match isolate_state.borrow_mut().futures.poll_next_unpin(cx) {
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
						let error =
							v8::String::new(&mut context_scope, &error.to_string()).unwrap();
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

			// Handle the output.
			let output = v8::Local::new(&mut context_scope, output.clone());
			match v8::Local::<v8::Promise>::try_from(output) {
				Err(_) => Poll::Ready(Ok::<_, anyhow::Error>(())),

				Ok(promise) => match promise.state() {
					v8::PromiseState::Pending => Poll::Pending,

					v8::PromiseState::Fulfilled => Poll::Ready(Ok(())),

					v8::PromiseState::Rejected => {
						let exception = promise.result(&mut context_scope);
						let exception = exception_to_string(
							&mut context_scope,
							&context_state.borrow(),
							exception,
						);
						Poll::Ready(Err(anyhow!("{exception}")))
					},
				},
			}
		}
	})
	.await?;

	// Retrieve the output.
	let output_hash = context_state
		.borrow_mut()
		.output
		.take()
		.context("The process did not return a value.")?;

	Ok(output_hash)
}

/// Load a module at the specified URL.
fn load_module<'s>(
	scope: &mut v8::HandleScope<'s>,
	url: &compiler::Url,
) -> Result<v8::Local<'s, v8::Module>> {
	// Get the context and context scope.
	let context = scope.get_current_context();
	let context_state = Rc::clone(
		context
			.get_slot::<Rc<RefCell<ContextState>>>(scope)
			.unwrap(),
	);

	// Return a cached module if this URL has already been loaded.
	if let Some(module) = context_state
		.borrow()
		.modules
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
	let script_id = context_state.borrow().modules.len().to_i32().unwrap() + 1;
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
	context_state.borrow().main_runtime_handle.spawn({
		let compiler = context_state.borrow().compiler.clone();
		let url = url.clone();
		async move {
			let source = match compiler.load(&url).await {
				Ok(source) => source,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};
			let transpile_output = match compiler.transpile(source.clone()).await {
				Ok(transpile_output) => transpile_output,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};
			let output = (
				source,
				transpile_output.transpiled_source,
				transpile_output.source_map,
			);
			sender.send(Ok(output)).unwrap();
		}
	});
	let (source, transpiled_source, source_map) = receiver
		.recv()
		.unwrap()
		.with_context(|| format!(r#"Failed to load from URL "{url}"."#))?;
	let source_map =
		SourceMap::from_slice(source_map.as_bytes()).context("Failed to parse the source map.")?;

	// Compile the module.
	let mut try_catch_scope = v8::TryCatch::new(scope);
	let module_source = v8::String::new(&mut try_catch_scope, &transpiled_source).unwrap();
	let module_source = v8::script_compiler::Source::new(module_source, Some(&origin));
	let module = v8::script_compiler::compile_module(&mut try_catch_scope, module_source);
	if try_catch_scope.has_caught() {
		let exception = try_catch_scope.exception().unwrap();
		let mut scope = v8::HandleScope::new(&mut try_catch_scope);
		let exception = exception_to_string(&mut scope, &context_state.borrow(), exception);
		bail!("{exception}");
	}
	let module = module.unwrap();
	drop(try_catch_scope);

	// Cache the module.
	context_state.borrow_mut().modules.push(Module {
		identity_hash: module.get_identity_hash(),
		module: v8::Global::new(scope, module),
		url: url.clone(),
		_source: source,
		_transpiled_source: Some(transpiled_source),
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
	let context_state = Rc::clone(
		context
			.get_slot::<Rc<RefCell<ContextState>>>(&mut scope)
			.unwrap(),
	);

	// Get the specifier.
	let specifier = specifier.to_rust_string_lossy(&mut scope);

	// Get the referrer URL.
	let referrer_identity_hash = referrer.get_identity_hash();
	let referrer = context_state
		.borrow()
		.modules
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
	context_state.borrow().main_runtime_handle.spawn({
		let compiler = context_state.borrow().compiler.clone();
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
	writeln!(string, "{}", message).unwrap();

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
			let line: u32 = stack_trace_frame.get_line_number().try_into().unwrap();
			let column: u32 = stack_trace_frame.get_column().try_into().unwrap();

			// Apply a source map if one is available.
			let (line, column) = if let Some(source_map) = context_state
				.modules
				.iter()
				.find(|module| module.url == url)
				.and_then(|module| module.source_map.as_ref())
			{
				let token = source_map.lookup_token(line - 1, column - 1).unwrap();
				let line = token.get_src_line() + 1;
				let column = token.get_src_col() + 1;
				(line, column)
			} else {
				(line, column)
			};

			// Write the URL, line, and column.
			write!(string, "{url}:{line}:{column}").unwrap();

			// Add a newline if this is not the last frame.
			if i < stack_trace.get_frame_count() - 1 {
				writeln!(string).unwrap();
			}
		}
	}

	string
}
