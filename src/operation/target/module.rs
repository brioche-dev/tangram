use super::context::{ContextState, Module};
use crate::compiler::ModuleIdentifier;
use anyhow::{bail, Context, Result};
use num::ToPrimitive;
use sourcemap::SourceMap;
use std::rc::Rc;

// async fn evaluate_module<'s>(
// 	scope: &mut v8::HandleScope<'s>,
// 	module: v8::Local<'s, v8::Module>,
// ) -> Result<v8::Local<'s, v8::Value>> {
// 	todo!();
// }

/// Load a module.
pub fn load_module<'s>(
	scope: &mut v8::HandleScope<'s>,
	module_identifier: &ModuleIdentifier,
) -> Result<v8::Local<'s, v8::Module>> {
	// Get the context and context scope.
	let context = scope.get_current_context();
	let context_state = Rc::clone(context.get_slot::<Rc<ContextState>>(scope).unwrap());

	// Return a cached module if this module has already been loaded.
	if let Some(module) = context_state
		.modules
		.borrow()
		.iter()
		.find(|module| &module.module_identifier == module_identifier)
	{
		let module = v8::Local::new(scope, &module.module);
		return Ok(module);
	}

	// Define the module's origin.
	let resource_name = v8::String::new(scope, &module_identifier.to_string()).unwrap();
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
		let module_identifier = module_identifier.clone();
		async move {
			// Load the module.
			let source = match compiler.load(&module_identifier).await {
				Ok(source) => source,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};

			// Transpile the module.
			let output = match compiler.transpile(source.clone()).await {
				Ok(transpile_output) => transpile_output,
				Err(error) => return sender.send(Err(error)).unwrap(),
			};

			// Send the output.
			let output = (source, output.transpiled, output.source_map);
			sender.send(Ok(output)).unwrap();
		}
	});
	let (source, transpiled, source_map) = receiver
		.recv()
		.unwrap()
		.with_context(|| format!(r#"Failed to load module "{module_identifier}"."#))?;
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
		let exception = super::exception::render(&mut scope, &context_state, exception);
		bail!("{exception}");
	}
	let module = module.unwrap();
	drop(try_catch_scope);

	// Cache the module.
	context_state.modules.borrow_mut().push(Module {
		identity_hash: module.get_identity_hash(),
		module: v8::Global::new(scope, module),
		module_identifier: module_identifier.clone(),
		source,
		_transpiled: Some(transpiled),
		source_map: Some(source_map),
	});

	Ok(module)
}

/// Implement V8's module resolution callback.
pub fn resolve_module_callback<'s>(
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

	// Get the referrer.
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
		.module_identifier
		.clone();

	// Resolve.
	let (sender, receiver) = std::sync::mpsc::channel();
	context_state.main_runtime_handle.spawn({
		let compiler = context_state.compiler.clone();
		let specifier = specifier.clone();
		let referrer = referrer.clone();
		async move {
			let module_identifier = compiler.resolve(&specifier, Some(&referrer)).await;
			sender.send(module_identifier).unwrap();
		}
	});
	let module_identifier = receiver.recv().unwrap().with_context(|| {
		format!(r#"Failed to resolve specifier "{specifier}" relative to referrer "{referrer:?}"."#)
	})?;

	// Load.
	let module = load_module(&mut scope, &module_identifier)
		.with_context(|| format!(r#"Failed to load module "{module_identifier}"."#))?;

	Ok(module)
}
