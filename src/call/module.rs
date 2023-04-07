use super::{
	context::await_value,
	isolate::THREAD_LOCAL_ISOLATE,
	state::{self, State},
};
use crate::{
	error::{Error, Result, WrapErr},
	instance::Instance,
	module::{self, Module},
};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::{rc::Rc, sync::Arc};

#[allow(clippy::await_holding_refcell_ref)]
pub async fn evaluate(
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
		let error = Error::from_exception(&mut try_catch_scope, &state, exception);
		return Err(error);
	}
	drop(try_catch_scope);

	// Evaluate the module.
	let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
	let module_output = module.evaluate(&mut try_catch_scope);
	let Some(module_output) = module_output else {
		let exception = try_catch_scope.exception().unwrap();
		let error = Error::from_exception(&mut try_catch_scope, &state, exception);
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

	// Get the instance.
	let tg = context.get_slot::<Arc<Instance>>(scope).unwrap().clone();

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
	tg.runtime_handle.spawn({
		let tg = tg.clone();
		let module = module.clone();
		async move {
			// Load the module.
			let text = match module.load(&tg).await {
				Ok(text) => text,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};

			// Transpile the module.
			let output = match Module::transpile(text.clone()).await {
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
		let error = Error::from_exception(&mut try_catch_scope, &state, exception);
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
			let exception = error.to_exception(&mut scope);
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

	// Get the instance.
	let tg = context
		.get_slot::<Arc<Instance>>(&mut scope)
		.unwrap()
		.clone();

	// Get the state.
	let state = context.get_slot::<Rc<State>>(&mut scope).unwrap().clone();

	// Get the specifier.
	let specifier = specifier.to_rust_string_lossy(&mut scope);
	let specifier: module::Specifier = specifier.parse()?;

	// Get the referrer.
	let referrer_identity_hash = referrer.get_identity_hash();
	let referrer = state
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
	tg.runtime_handle.spawn({
		let tg = tg.clone();
		let specifier = specifier.clone();
		let referrer = referrer.clone();
		async move {
			let module = referrer.resolve(&tg, &specifier).await;
			sender.send(module).unwrap();
		}
	});
	let module = receiver.recv().unwrap().wrap_err_with(|| {
		format!(r#"Failed to resolve specifier "{specifier}" relative to referrer "{referrer}"."#)
	})?;

	// Load.
	let module = load_module(&mut scope, &module).wrap_err(r#"Failed to load the module."#)?;

	Ok(module)
}
