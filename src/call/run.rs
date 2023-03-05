use super::{isolate::THREAD_LOCAL_ISOLATE, Call};
use crate::{module, value::Value, Instance};
use anyhow::{Context, Result};
use std::{rc::Rc, sync::Arc};

impl Instance {
	// Run a call.
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
			.context("Failed to join the task.")?
			.unwrap();

		Ok(output)
	}
}

#[allow(clippy::await_holding_refcell_ref)]
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

	// Move the module to the context.
	let module = v8::Local::new(&mut context_scope, module);

	// Get the module namespace.
	let namespace = module.get_module_namespace();
	let namespace = namespace.to_object(&mut context_scope).unwrap();

	// Get the function.
	let function_name_string = v8::String::new(&mut context_scope, &call.function.name).unwrap();
	let function: v8::Local<v8::Function> = namespace
		.get(&mut context_scope, function_name_string.into())
		.context("Failed to get the export.")?
		.try_into()
		.context("The export must be an object.")?;
	let run_string = v8::String::new(&mut context_scope, "run").unwrap();
	let run: v8::Local<v8::Function> = function
		.get(&mut context_scope, run_string.into())
		.context(r#"The export must be a tangram function."#)?
		.try_into()
		.context(r#"The value for the key "run" must be a function."#)?;

	// Serialize the context to v8.
	let serialized_context = serde_v8::to_v8(&mut context_scope, &call.context)
		.context("Failed to serialize the context.")?;

	// Serialize the args to v8.
	let serialized_args =
		serde_v8::to_v8(&mut context_scope, &call.args).context("Failed to serialize the args.")?;

	// Call the function.
	let output = run
		.call(
			&mut context_scope,
			function.into(),
			&[serialized_args, serialized_context],
		)
		.unwrap();

	// Make the output and context global.
	let output = v8::Global::new(&mut context_scope, output);
	let context = v8::Global::new(&mut context_scope, context);

	// Exit the context.
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
	let output = serde_v8::from_v8(&mut context_scope, output)?;

	// Exit the context.
	drop(context_scope);
	drop(handle_scope);
	drop(isolate);

	Ok(output)
}
