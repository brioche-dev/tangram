use super::{isolate::THREAD_LOCAL_ISOLATE, state::State, Call};
use crate::{
	error::{Error, Result, WrapErr},
	module,
	value::Value,
	Instance,
};
use std::{rc::Rc, sync::Arc};

impl Instance {
	// Run a call.
	#[tracing::instrument(skip(self), ret)]
	pub async fn run_call(self: &Arc<Self>, call: &Call) -> Result<Value> {
		// Run the call on the local pool because it is a `!Send` future.
		let output = self
			.local_pool_handle
			.spawn_pinned({
				let tg = Arc::clone(self);
				let call = call.clone();
				move || async move { run_call_inner(tg, &call).await }
			})
			.await
			.map_err(Error::other)
			.wrap_err("Failed to join the task.")??;

		Ok(output)
	}
}

#[allow(clippy::await_holding_refcell_ref)]
#[tracing::instrument(skip_all)]
async fn run_call_inner(tg: Arc<Instance>, call: &Call) -> Result<Value> {
	// Create the context.
	let context = super::context::new(Arc::clone(&tg));

	// Create the module identifier.
	let module_identifier = module::Identifier::for_root_module_in_package_instance(
		call.function.package_instance_hash,
	);

	// Evaluate the module.
	let (module, _) = super::module::evaluate(context.clone(), &module_identifier).await?;

	// Enter the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Local::new(&mut handle_scope, context.clone());
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Get the state.
	let state = Rc::clone(context.get_slot::<Rc<State>>(&mut context_scope).unwrap());

	// Create a try catch scope.
	let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);

	// Move the module to the context.
	let module = v8::Local::new(&mut try_catch_scope, module);

	// Get the module namespace.
	let namespace = module
		.get_module_namespace()
		.to_object(&mut try_catch_scope)
		.unwrap();

	// Get the function.
	let function_name_string = v8::String::new(&mut try_catch_scope, &call.function.name).unwrap();
	let function: v8::Local<v8::Function> = namespace
		.get(&mut try_catch_scope, function_name_string.into())
		.wrap_err("Failed to get the export.")?
		.try_into()
		.map_err(Error::other)
		.wrap_err("The export must be an object.")?;
	let run_string = v8::String::new(&mut try_catch_scope, "run").unwrap();
	let run: v8::Local<v8::Function> = function
		.get(&mut try_catch_scope, run_string.into())
		.wrap_err(r#"The export must be a tangram function."#)?
		.try_into()
		.map_err(Error::other)
		.wrap_err(r#"The value for the key "run" must be a function."#)?;

	// Serialize the context to v8.
	let serialized_context = serde_v8::to_v8(&mut try_catch_scope, &call.context)
		.map_err(Error::other)
		.wrap_err("Failed to serialize the context.")?;

	// Serialize the args to v8.
	let serialized_args = serde_v8::to_v8(&mut try_catch_scope, &call.args)
		.map_err(Error::other)
		.wrap_err("Failed to serialize the args.")?;

	// Call the function.
	let output = run.call(
		&mut try_catch_scope,
		function.into(),
		&[serialized_context, serialized_args],
	);

	// Handle an exception.
	if try_catch_scope.has_caught() {
		let exception = try_catch_scope.exception().unwrap();
		let error = Error::from_exception(&mut try_catch_scope, &state, exception);
		return Err(error);
	}
	let output = output.unwrap();

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

	// Deserialize the output from v8.
	let output = serde_v8::from_v8(&mut context_scope, output).map_err(Error::other)?;

	// Exit the context.
	drop(context_scope);
	drop(handle_scope);
	drop(isolate);

	Ok(output)
}
