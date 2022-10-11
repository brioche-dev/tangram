use self::module_loader::{ModuleLoader, TANGRAM_MODULE_SCHEME};
use crate::{
	builder::Shared,
	expression::{self, Expression, Js},
	hash::Hash,
};
use anyhow::{bail, Context, Result};
use deno_core::{serde_v8, v8, JsRuntime};
use std::{cell::RefCell, convert::TryInto, future::Future, rc::Rc, sync::Arc};
use url::Url;

mod module_loader;

impl Shared {
	pub(super) async fn evaluate_js(&self, hash: Hash, js: &Js) -> Result<Hash> {
		// Get a handle to the current tokio runtime.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Run the js process on the local task pool.
		let output_hash = self
			.local_pool_handle
			.spawn_pinned({
				let builder = self.clone();
				let js = js.clone();
				move || async move { run_js_process(builder, main_runtime_handle, &js).await }
			})
			.await
			.unwrap()?;

		// Evaluate the expression.
		let output_hash = self
			.evaluate(output_hash, hash)
			.await
			.context("Failed to evaluate the expression returned by the JS process.")?;

		Ok(output_hash)
	}
}

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/snapshot"));

#[derive(Clone)]
struct OpState {
	builder: crate::builder::Shared,
	main_runtime_handle: tokio::runtime::Handle,
}

#[allow(clippy::too_many_lines)]
async fn run_js_process(
	builder: crate::builder::Shared,
	main_runtime_handle: tokio::runtime::Handle,
	js: &expression::Js,
) -> Result<Hash> {
	// Build the tangram extension.
	let tangram_extension = deno_core::Extension::builder()
		.ops(vec![
			op_tangram_print::decl(),
			op_tangram_add_blob::decl(),
			op_tangram_get_blob::decl(),
			op_tangram_add_expression::decl(),
			op_tangram_get_expression::decl(),
			op_tangram_evaluate::decl(),
		])
		.state({
			let main_runtime_handle = main_runtime_handle.clone();
			let builder = builder.clone();
			move |state| {
				let builder = builder.clone();
				state.put(Arc::new(OpState {
					builder,
					main_runtime_handle: main_runtime_handle.clone(),
				}));
				Ok(())
			}
		})
		.build();

	// Create the module loader.
	let module_loader = Rc::new(ModuleLoader::new(
		builder.clone(),
		main_runtime_handle.clone(),
	));

	// Create the js runtime.
	let mut runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
		source_map_getter: Some(
			Box::new(Rc::clone(&module_loader)) as Box<dyn deno_core::SourceMapGetter>
		),
		module_loader: Some(Rc::clone(&module_loader) as Rc<dyn deno_core::ModuleLoader>),
		extensions: vec![tangram_extension],
		startup_snapshot: Some(deno_core::Snapshot::Static(SNAPSHOT)),
		..Default::default()
	});

	// Create the module URL.
	let mut module_url = format!("{TANGRAM_MODULE_SCHEME}://{}", js.package);

	// Add the module path.
	module_url.push('/');
	module_url.push_str(js.path.as_str());

	// Parse the module URL.
	let module_url = Url::parse(&module_url).unwrap();

	// Load the module.
	let module_id = runtime.load_side_module(&module_url, None).await?;
	let evaluate_receiver = runtime.mod_evaluate(module_id);
	runtime.run_event_loop(false).await?;
	evaluate_receiver.await.unwrap()?;

	// Move the args to v8.
	let args = builder
		.get_expression_local(js.args)
		.context("Failed to get the args expression.")?;
	let args = args.as_array().context("The args must be an array.")?;
	let mut arg_values = Vec::new();
	for arg in args {
		// Create a try catch scope to call Tangram.toJson.
		let mut scope = runtime.handle_scope();
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		let arg = builder.get_expression_local(*arg)?;
		let arg = serde_v8::to_v8(&mut try_catch_scope, arg)
			.context("Failed to move the args expression to v8.")?;

		// Retrieve Tangram.fromJson.
		let from_json_function: v8::Local<v8::Function> =
			JsRuntime::grab_global(&mut try_catch_scope, "Tangram.fromJson")
				.context("Failed to get Tangram.fromJson.")?;

		// Call Tangram.fromJson.
		let undefined = v8::undefined(&mut try_catch_scope);
		let output = from_json_function.call(&mut try_catch_scope, undefined.into(), &[arg]);

		// If an exception was caught, return an error with an error message.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
			bail!(error);
		}

		// If there was no caught exception then retrieve the return value.
		let output = output.unwrap();

		// Move the return value to the global scope.
		let output = v8::Global::new(&mut try_catch_scope, output);
		drop(try_catch_scope);
		drop(scope);

		// Run the event loop to completion.
		runtime.run_event_loop(false).await?;

		// Retrieve the output.
		let mut scope = runtime.handle_scope();
		let output = v8::Local::new(&mut scope, output);
		let output = if output.is_promise() {
			let promise: v8::Local<v8::Promise> = output.try_into().unwrap();
			promise.result(&mut scope)
		} else {
			output
		};

		// Move the output to the global scope.
		let output = v8::Global::new(&mut scope, output);

		drop(scope);

		arg_values.push(output);
	}

	// Retrieve the specified export from the module.
	let module_namespace = runtime.get_module_namespace(module_id)?;
	let mut scope = runtime.handle_scope();
	let module_namespace = v8::Local::<v8::Object>::new(&mut scope, module_namespace);
	let export_name = js.name.clone();
	let export_literal = v8::String::new(&mut scope, &export_name).unwrap();
	let export: v8::Local<v8::Function> = module_namespace
		.get(&mut scope, export_literal.into())
		.with_context(|| {
			format!(r#"Failed to get the export "{export_name}" from the module "{module_url}"."#)
		})?
		.try_into()
		.with_context(|| {
			format!(
				r#"The export "{export_name}" from the module "{module_url}" must be a function."#
			)
		})?;

	// Create a scope to call the export.
	let mut try_catch_scope = v8::TryCatch::new(&mut scope);

	// Move the arg values to the try catch scope.
	let arg_values = arg_values
		.iter()
		.map(|arg| v8::Local::new(&mut try_catch_scope, arg))
		.collect::<Vec<_>>();

	// Call the specified export.
	let undefined = v8::undefined(&mut try_catch_scope);
	let output = export.call(&mut try_catch_scope, undefined.into(), &arg_values);

	// If an exception was caught, return an error with an error message.
	if try_catch_scope.has_caught() {
		let exception = try_catch_scope.exception().unwrap();
		let mut scope = v8::HandleScope::new(&mut try_catch_scope);
		let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
		bail!(error);
	}

	// If there was no caught exception then retrieve the return value.
	let output = output.unwrap();

	// Move the return value to the global scope.
	let output = v8::Global::new(&mut try_catch_scope, output);
	drop(try_catch_scope);
	drop(scope);

	// Run the event loop to completion.
	runtime.run_event_loop(false).await?;

	// Retrieve the return value.
	let mut scope = runtime.handle_scope();
	let output = v8::Local::new(&mut scope, output);
	let output = if output.is_promise() {
		let promise: v8::Local<v8::Promise> = output.try_into().unwrap();
		promise.result(&mut scope)
	} else {
		output
	};

	// Move the return value to the global scope.
	let output = v8::Global::new(&mut scope, output);
	drop(scope);

	// Create a try catch scope to call Tangram.toJson.
	let mut scope = runtime.handle_scope();
	let mut try_catch_scope = v8::TryCatch::new(&mut scope);

	// Move the output to the try catch scope.
	let output = v8::Local::new(&mut try_catch_scope, output);

	// Retrieve Tangram.toJson.
	let to_json_function: v8::Local<v8::Function> =
		JsRuntime::grab_global(&mut try_catch_scope, "Tangram.toJson")
			.context("Failed to get Tangram.toJson.")?;

	// Call Tangram.toJson.
	let undefined = v8::undefined(&mut try_catch_scope);
	let output = to_json_function.call(&mut try_catch_scope, undefined.into(), &[output]);

	// If an exception was caught, return an error with an error message.
	if try_catch_scope.has_caught() {
		let exception = try_catch_scope.exception().unwrap();
		let mut scope = v8::HandleScope::new(&mut try_catch_scope);
		let error = deno_core::error::JsError::from_v8_exception(&mut scope, exception);
		bail!(error);
	}

	// If there was no caught exception then retrieve the return value.
	let output = output.unwrap();

	// Move the return value to the global scope.
	let output = v8::Global::new(&mut try_catch_scope, output);
	drop(try_catch_scope);
	drop(scope);

	// Run the event loop to completion.
	runtime.run_event_loop(false).await?;

	// Retrieve the output.
	let mut scope = runtime.handle_scope();
	let output = v8::Local::new(&mut scope, output);
	let output = if output.is_promise() {
		let promise: v8::Local<v8::Promise> = output.try_into().unwrap();
		promise.result(&mut scope)
	} else {
		output
	};

	// Deserialize the output.
	let expression: Expression = serde_v8::from_v8(&mut scope, output)?;

	// Add the expression.
	let hash = builder.add_expression(&expression).await?;

	Ok(hash)
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn op_tangram_print(string: String) -> Result<(), deno_core::error::AnyError> {
	println!("{string}");
	Ok(())
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_add_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	blob: String,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |builder| async move {
		let hash = builder.add_blob(blob.as_bytes()).await?;
		Ok::<_, anyhow::Error>(hash)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_get_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<String, deno_core::error::AnyError> {
	op(state, |builder| async move {
		let path = builder.get_blob(hash).await?;
		let string = tokio::fs::read_to_string(&path).await?;
		Ok::<_, anyhow::Error>(string)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_add_expression(
	state: Rc<RefCell<deno_core::OpState>>,
	expression: Expression,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |builder| async move {
		let hash = builder.add_expression(&expression).await?;
		Ok::<_, anyhow::Error>(hash)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_get_expression(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Option<Expression>, deno_core::error::AnyError> {
	op(state, |builder| async move {
		let expression = builder.try_get_expression_local(hash)?;
		Ok::<_, anyhow::Error>(expression)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_evaluate(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |builder| async move {
		let output = builder.evaluate(hash, hash).await?;
		Ok::<_, anyhow::Error>(output)
	})
	.await
}

async fn op<T, F, Fut>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<T, deno_core::error::AnyError>
where
	T: 'static + Send,
	F: FnOnce(crate::builder::Shared) -> Fut,
	Fut: 'static + Send + Future<Output = Result<T, deno_core::error::AnyError>>,
{
	let state = {
		let state = state.borrow();
		let state = state.borrow::<Arc<OpState>>();
		Arc::clone(state)
	};
	let output = state
		.main_runtime_handle
		.spawn({
			let builder = state.builder.clone();
			f(builder)
		})
		.await
		.unwrap()?;
	Ok(output)
}
