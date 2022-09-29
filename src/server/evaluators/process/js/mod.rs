use self::module_loader::{ModuleLoader, TANGRAM_MODULE_SCHEME};
use super::Process;
use crate::{
	expression::{Expression, JsProcess},
	hash::Hash,
	server::Server,
};
use anyhow::{anyhow, bail, Context, Result};
use deno_core::{serde_v8, v8};
use futures::{executor::block_on, future::try_join_all};
use std::{cell::RefCell, convert::TryInto, fmt::Write, future::Future, rc::Rc, sync::Arc};
use url::Url;

mod module_loader;

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/snapshot"));

#[derive(Clone)]
struct OpState {
	server: Arc<Server>,
	main_runtime_handle: tokio::runtime::Handle,
}

impl Process {
	pub(super) async fn evaluate_js_process(
		&self,
		server: &Arc<Server>,
		hash: Hash,
		process: &JsProcess,
	) -> Result<Hash> {
		// Get a handle to the current tokio runtime.
		let main_runtime_handle = tokio::runtime::Handle::current();

		// Run the js process on the local task pool.
		let output_hash = self
			.local_pool_handle
			.spawn_pinned({
				let process = process.clone();
				let server = Arc::clone(server);
				move || async move { Self::run_js_process(server, main_runtime_handle, &process).await }
			})
			.await
			.unwrap()?;

		// Evaluate the expression.
		let output_hash = server
			.evaluate(output_hash, hash)
			.await
			.context("Failed to evaluate the expression returned by the JS process.")?;

		Ok(output_hash)
	}

	#[allow(clippy::too_many_lines)]
	async fn run_js_process(
		server: Arc<Server>,
		main_runtime_handle: tokio::runtime::Handle,
		process: &JsProcess,
	) -> Result<Hash> {
		// Build the tangram extension.
		let tangram_extension = deno_core::Extension::builder()
			.ops(vec![
				op_tangram_add_expression::decl(),
				op_tangram_get_expression::decl(),
				op_tangram_add_blob::decl(),
				op_tangram_get_blob::decl(),
				op_tangram_console_log::decl(),
				op_tangram_evaluate::decl(),
			])
			.state({
				let server = Arc::clone(&server);
				let main_runtime_handle = main_runtime_handle.clone();
				move |state| {
					state.put(Arc::new(OpState {
						server: Arc::clone(&server),
						main_runtime_handle: main_runtime_handle.clone(),
					}));
					Ok(())
				}
			})
			.build();

		// Create the module loader.
		let module_loader = Rc::new(ModuleLoader::new(
			Arc::clone(&server),
			main_runtime_handle.clone(),
		));

		// Create the js runtime.
		let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
			source_map_getter: Some(
				Box::new(Rc::clone(&module_loader)) as Box<dyn deno_core::SourceMapGetter>
			),
			module_loader: Some(Rc::clone(&module_loader) as Rc<dyn deno_core::ModuleLoader>),
			extensions: vec![tangram_extension],
			startup_snapshot: Some(deno_core::Snapshot::Static(SNAPSHOT)),
			..Default::default()
		});

		// Create the module URL.
		let mut module_url = format!("{TANGRAM_MODULE_SCHEME}://{}", process.artifact);

		// Add the module path if necessary.
		if let Some(path) = &process.path {
			module_url.push('/');
			module_url.push_str(path.as_str());
		}

		// Add the lockfile if necessary.
		if let Some(lockfile) = process.lockfile.as_ref() {
			let lockfile_hash = module_loader.add_lockfile(lockfile.clone());
			write!(module_url, "?lockfile_hash={lockfile_hash}").unwrap();
		}

		// Parse the module URL.
		let module_url = Url::parse(&module_url).unwrap();

		// Load the module.
		let module_id = js_runtime.load_side_module(&module_url, None).await?;
		let evaluate_receiver = js_runtime.mod_evaluate(module_id);
		js_runtime.run_event_loop(false).await?;
		evaluate_receiver.await.unwrap()?;

		// Retrieve the specified export from the module.
		let module_namespace = js_runtime.get_module_namespace(module_id)?;
		let mut scope = js_runtime.handle_scope();
		let module_namespace = v8::Local::<v8::Object>::new(&mut scope, module_namespace);
		let export_name = process.export.clone();
		let export_literal = v8::String::new(&mut scope, &export_name).unwrap();
		let export: v8::Local<v8::Function> = module_namespace
			.get(&mut scope, export_literal.into())
			.ok_or_else(|| {
				anyhow!(
					r#"Failed to get the export "{export_name}" from the module "{module_url}"."#
				)
			})?
			.try_into()
			.with_context(|| {
				format!(
					r#"The export "{export_name}" from the module "{module_url}" must be a function."#
				)
			})?;

		// Create a scope to call the export.
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);
		let undefined = v8::undefined(&mut try_catch_scope);

		// Evaluate the args and move them to v8.
		let args = server.get_expression(process.args).await?;
		let args = match args {
			Expression::Array(array) => array,
			_ => bail!("Args must be an array."),
		};
		let args = try_join_all(args.iter().map(|arg| {
			let server = Arc::clone(&server);
			async move {
				let expression = server.get_expression(*arg).await?;
				match expression {
					Expression::String(string) => Ok::<_, anyhow::Error>(string),
					_ => bail!("Args must evaluate to strings."),
				}
			}
		}))
		.await?;

		let args = args
			.iter()
			.map(|arg| {
				let arg = serde_v8::to_v8(&mut try_catch_scope, arg)?;
				Ok(arg)
			})
			.collect::<Result<Vec<_>>>()?;

		// Call the specified export.
		let output = export.call(&mut try_catch_scope, undefined.into(), &args);

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
		js_runtime.run_event_loop(false).await?;

		// Retrieve the return value.
		let mut scope = js_runtime.handle_scope();
		let output = v8::Local::new(&mut scope, output);
		let output = if output.is_promise() {
			let promise: v8::Local<v8::Promise> = output.try_into().unwrap();
			promise.result(&mut scope)
		} else {
			output
		};

		// If the output is null, create a tangram null expression.
		if output.is_null() {
			let hash = server.add_expression(&Expression::Null).await?;
			return Ok(hash);
		}

		// If the output is a string, create a tangram string expression.
		if output.is_string() {
			let value: String = serde_v8::from_v8(&mut scope, output).unwrap();
			let hash = server
				.add_expression(&Expression::String(value.into()))
				.await?;
			return Ok(hash);
		}

		// If the output is a number, create a tangram number expression.
		if output.is_number() {
			let value: f64 = serde_v8::from_v8(&mut scope, output).unwrap();
			let hash = server.add_expression(&Expression::Number(value)).await?;
			return Ok(hash);
		}

		// If the output is a bool, create a tangram bool expression.
		if output.is_boolean() {
			let value: bool = serde_v8::from_v8(&mut scope, output).unwrap();
			let hash = server.add_expression(&Expression::Bool(value)).await?;
			return Ok(hash);
		}

		// The output must be an object.
		let output: v8::Local<v8::Object> = output
			.try_into()
			.context("The returned value must be a Tangram Expression or Tangram Hash Object.")?;

		// If the output has a "hash" property and it is not a function, get it.
		let hash_property_name = v8::String::new(&mut scope, "hash").unwrap();
		let hash_value: Option<v8::Local<v8::Value>> =
			output.get(&mut scope, hash_property_name.into());
		if let Some(hash_value) = hash_value {
			if !hash_value.is_function() {
				let hash = serde_v8::from_v8(&mut scope, hash_value)?;
				return Ok(hash);
			}
		}

		// Otherwise, it is a Tangram JS Expression Object that we need to call the hash function on.
		let hash_fn_name = v8::String::new(&mut scope, "hash").unwrap();
		let hash_fn: v8::Local<v8::Function> = output
			.get(&mut scope, hash_fn_name.into())
			.ok_or_else(|| anyhow!(r#"The returned object must be a tangram expression"#))?
			.try_into()
			.context(
				r#"The returned object must be a tangram expression object with a function named "hash"."#,
			)?;

		// Create a scope to call the export.
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		// Call the specified export.
		let output = hash_fn.call(&mut try_catch_scope, output.into(), &[]);

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
		js_runtime.run_event_loop(false).await?;

		// Retrieve the return value.
		let mut scope = js_runtime.handle_scope();
		let output = v8::Local::new(&mut scope, output);
		let output = if output.is_promise() {
			let promise: v8::Local<v8::Promise> = output.try_into().unwrap();
			promise.result(&mut scope)
		} else {
			output
		};
		let output: v8::Local<v8::Object> = output.try_into().unwrap();
		let hash_property = v8::String::new(&mut scope, "hash").unwrap();
		let hash = output.get(&mut scope, hash_property.into()).unwrap();
		let hash = serde_v8::from_v8(&mut scope, hash)?;
		Ok(hash)
	}
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_add_expression(
	state: Rc<RefCell<deno_core::OpState>>,
	expression: Expression,
) -> Result<Hash, deno_core::error::AnyError> {
	block_on(op(state, |server| async move {
		server.add_expression(&expression).await
	}))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_get_expression(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Option<Expression>, deno_core::error::AnyError> {
	block_on(op(state, |server| async move {
		server.try_get_expression(hash).await
	}))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_add_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	blob: String,
) -> Result<Hash, deno_core::error::AnyError> {
	block_on(op(state, |server| async move {
		server.add_blob(blob.as_bytes()).await
	}))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_get_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	_hash: Hash,
) -> Result<Option<Vec<u8>>, deno_core::error::AnyError> {
	block_on(op(state, |_| async move { todo!() }))
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
fn op_tangram_console_log(args: Vec<serde_json::Value>) -> Result<(), deno_core::error::AnyError> {
	let len = args.len();
	for (i, arg) in args.into_iter().enumerate() {
		print!("{arg}");
		if i != len - 1 {
			print!(" ");
		}
	}
	println!();
	Ok(())
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_evaluate(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Hash, deno_core::error::AnyError> {
	op(
		state,
		|server| async move { server.evaluate(hash, hash).await },
	)
	.await
}

async fn op<T, F, Fut>(
	state: Rc<RefCell<deno_core::OpState>>,
	f: F,
) -> Result<T, deno_core::error::AnyError>
where
	T: 'static + Send,
	F: FnOnce(Arc<Server>) -> Fut,
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
			let server = Arc::clone(&state.server);
			f(server)
		})
		.await
		.unwrap()?;
	Ok(output)
}
