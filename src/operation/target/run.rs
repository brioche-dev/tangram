use super::{
	context::{await_value, create_context},
	isolate::THREAD_LOCAL_ISOLATE,
	module::evaluate_module,
	Target,
};
use crate::{compiler::ModuleIdentifier, value::Value, Cli};
use anyhow::{Context, Result};
use std::rc::Rc;

impl Cli {
	pub async fn run_target(&self, target: &Target) -> Result<Value> {
		// Run the target on the local pool because it is a `!Send` future.
		let output = self
			.inner
			.local_pool_handle
			.spawn_pinned({
				let cli = self.clone();
				let target = target.clone();
				move || async move { run_target_inner(cli, &target).await }
			})
			.await
			.context("Failed to join the task.")?
			.context("Failed to run the target.")?;

		Ok(output)
	}
}

async fn run_target_inner(cli: Cli, target: &Target) -> Result<Value> {
	// Create the context.
	let (context, state) = create_context(cli.clone());

	// Create the module identifier.
	let module_identifier = ModuleIdentifier::new_hash(target.package, "package.tg".into());

	// Evaluate the module.
	let (module, _) = evaluate_module(context.clone(), &module_identifier).await?;

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

	// Get the target export.
	let target_name_string = v8::String::new(&mut context_scope, &target.name).unwrap();
	let target_export = namespace
		.get(&mut context_scope, target_name_string.into())
		.context("Failed to get the target export.")?;

	// Get the run function.
	let target_export = <v8::Local<v8::Object>>::try_from(target_export)
		.context("The target export must be an object.")?;
	let run_string = v8::String::new(&mut context_scope, "run").unwrap();
	let target_export_run_function = target_export
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

	// Call the run function.
	let undefined = v8::undefined(&mut context_scope);
	let output = target_export_run_function
		.call(&mut context_scope, undefined.into(), &args)
		.unwrap();

	// Make the output and context global.
	let output = v8::Global::new(&mut context_scope, output);
	let context = v8::Global::new(&mut context_scope, context);

	// Exit the context.
	drop(context_scope);
	drop(handle_scope);
	drop(isolate);

	// Await the output.
	let output = await_value(context.clone(), Rc::clone(&state), output).await?;

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
