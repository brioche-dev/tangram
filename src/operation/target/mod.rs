use self::{
	context::{await_value, create_context, ContextState, FutureOutput},
	isolate::THREAD_LOCAL_ISOLATE,
	module::{load_module, resolve_module_callback},
};
use crate::{compiler::ModuleIdentifier, package::PackageHash, value::Value, Cli};
use anyhow::{bail, Context, Result};
use std::rc::Rc;

mod context;
mod exception;
mod isolate;
mod module;
mod syscall;

#[derive(
	Clone, Debug, buffalo::Deserialize, buffalo::Serialize, serde::Deserialize, serde::Serialize,
)]
pub struct Target {
	#[buffalo(id = 0)]
	pub package: PackageHash,

	#[buffalo(id = 1)]
	pub name: String,

	#[buffalo(id = 2)]
	pub args: Vec<Value>,
}

impl Cli {
	pub(super) async fn run_target(&self, target: &Target) -> Result<Value> {
		// Run the target on the local pool because it is a `!Send` future.
		let output = self
			.state
			.local_pool_handle
			.spawn_pinned({
				let main_runtime_handle = tokio::runtime::Handle::current();
				let cli = self.clone();
				let target = target.clone();
				move || async move { run_target_inner(cli, main_runtime_handle, &target).await }
			})
			.await
			.context("Failed to join the task.")?
			.context("Failed to run the target.")?;

		Ok(output)
	}
}

#[allow(clippy::too_many_lines)]
async fn run_target_inner(
	cli: Cli,
	main_runtime_handle: tokio::runtime::Handle,
	target: &Target,
) -> Result<Value> {
	let (context, context_state) = create_context(cli.clone(), main_runtime_handle);

	// Get the package's entrypoint path.
	let entrypoint_path = cli
		.get_package_entrypoint_path(target.package)
		.context("Failed to retrieve the package entrypoint.")?
		.context("The package must have an entrypoint.")?;

	// Create the module identifier.
	let module_identifier = ModuleIdentifier::new_hash(target.package, entrypoint_path);

	// Evaluate the module.
	let (module, module_output) = {
		// Enter the context.
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, context.clone());
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Load the module.
		let module = load_module(&mut context_scope, &module_identifier)?;

		// Instantiate the module.
		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
		let output = module.instantiate_module(&mut try_catch_scope, resolve_module_callback);
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let exception = self::exception::render(&mut scope, &context_state, exception);
			bail!("{exception}");
		}
		output.unwrap();
		drop(try_catch_scope);

		// Evaluate the module.
		let mut try_catch_scope = v8::TryCatch::new(&mut context_scope);
		let module_output = module.evaluate(&mut try_catch_scope);
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let exception =
				self::exception::render(&mut try_catch_scope, &context_state, exception);
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

	// Enter the context.
	let output = {
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
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
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
