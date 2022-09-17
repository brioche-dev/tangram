use self::module_loader::{ModuleLoader, TANGRAM_MODULE_SCHEME};
use super::Process;
use crate::{
	expression::{self, Artifact, Dependency, Directory, Expression, File, JsProcess, Symlink},
	hash::Hash,
	lockfile::Lockfile,
	server::Server,
};
use anyhow::{anyhow, bail, Context, Result};
use camino::Utf8PathBuf;
use deno_core::{serde_v8, v8};
use futures::future::try_join_all;
use itertools::Itertools;
use std::{
	cell::RefCell, collections::BTreeMap, convert::TryInto, fmt::Write, future::Future, rc::Rc,
	sync::Arc,
};
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
				// Expression ops.
				op_tangram_null::decl(),
				op_tangram_bool::decl(),
				op_tangram_number::decl(),
				op_tangram_string::decl(),
				op_tangram_artifact::decl(),
				op_tangram_directory::decl(),
				op_tangram_file::decl(),
				op_tangram_symlink::decl(),
				op_tangram_dependency::decl(),
				op_tangram_path::decl(),
				op_tangram_template::decl(),
				op_tangram_fetch::decl(),
				op_tangram_process::decl(),
				op_tangram_target::decl(),
				op_tangram_array::decl(),
				op_tangram_map::decl(),
				// Other ops.
				op_tangram_blob::decl(),
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

		// Get the path expression for the module.
		let module = match server.try_get_expression(process.module).await?.unwrap() {
			Expression::Path(module) => module,
			_ => bail!("The module must be a path."),
		};

		// Get the module's artifact.
		let module_artifact = match server.try_get_expression(module.artifact).await?.unwrap() {
			Expression::Artifact(artifact) => artifact,
			_ => bail!("Module artifact must be an artifact."),
		};

		// Create the module URL.
		let mut module_url = format!("{TANGRAM_MODULE_SCHEME}://{}", module_artifact.hash);

		// Add the module path if necessary.
		if let Some(path) = module.path {
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
		let args = try_join_all(process.args.iter().map(|arg| {
			let server = Arc::clone(&server);
			async move {
				let expression = server.get_expression(*arg).await?;
				match expression {
					Expression::String(s) => Ok::<_, anyhow::Error>(s),
					_ => bail!(anyhow!("Args must evaluate to strings.")),
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
		let output = serde_v8::from_v8(&mut scope, output)?;

		Ok(output)
	}
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_null(
	state: Rc<RefCell<deno_core::OpState>>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Null).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_bool(
	state: Rc<RefCell<deno_core::OpState>>,
	value: bool,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Bool(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_number(
	state: Rc<RefCell<deno_core::OpState>>,
	value: f64,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Number(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_string(
	state: Rc<RefCell<deno_core::OpState>>,
	value: String,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::String(value.into()))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_artifact(
	state: Rc<RefCell<deno_core::OpState>>,
	hash: Hash,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Artifact(Artifact { hash }))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_directory(
	state: Rc<RefCell<deno_core::OpState>>,
	entries: BTreeMap<String, Hash>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Directory(Directory { entries }))
			.await
	})
	.await
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum FileBlob {
	String(String),
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FileOptions {
	executable: Option<bool>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_file(
	state: Rc<RefCell<deno_core::OpState>>,
	blob_hash: Hash,
	options: Option<FileOptions>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		let executable = options
			.and_then(|options| options.executable)
			.unwrap_or(false);
		let output = server
			.add_expression(&Expression::File(File {
				blob_hash,
				executable,
			}))
			.await?;
		Ok(output)
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_symlink(
	state: Rc<RefCell<deno_core::OpState>>,
	target: Utf8PathBuf,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Symlink(Symlink { target }))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_dependency(
	state: Rc<RefCell<deno_core::OpState>>,
	artifact: Artifact,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Dependency(Dependency { artifact }))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_path(
	state: Rc<RefCell<deno_core::OpState>>,
	artifact: Hash,
	path: Option<Utf8PathBuf>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Path(expression::Path {
				artifact,
				path: path.map(Into::into),
			}))
			.await
	})
	.await
}

#[derive(serde::Deserialize)]
struct TemplateArgs {
	strings: Vec<String>,
	placeholders: Vec<Hash>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_template(
	state: Rc<RefCell<deno_core::OpState>>,
	args: TemplateArgs,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		let components = try_join_all(args.strings.into_iter().map(|string| async {
			server
				.add_expression(&Expression::String(string.into()))
				.await
		}))
		.await?
		.into_iter()
		.interleave(args.placeholders)
		.collect();
		let template = crate::expression::Template { components };
		let output = server
			.add_expression(&Expression::Template(template))
			.await?;
		Ok(output)
	})
	.await
}

#[derive(serde::Deserialize)]
struct FetchArgs {
	url: Url,
	hash: Option<Hash>,
	unpack: bool,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_fetch(
	state: Rc<RefCell<deno_core::OpState>>,
	args: FetchArgs,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, move |server| async move {
		server
			.add_expression(&Expression::Fetch(crate::expression::Fetch {
				url: args.url,
				hash: args.hash,
				unpack: args.unpack,
			}))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_process(
	state: Rc<RefCell<deno_core::OpState>>,
	process: expression::Process,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, move |server| async move {
		server.add_expression(&Expression::Process(process)).await
	})
	.await
}

#[derive(serde::Deserialize)]
struct TargetArgs {
	lockfile: Option<Lockfile>,
	package: Artifact,
	name: String,
	#[serde(default)]
	args: Vec<Hash>,
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_target(
	state: Rc<RefCell<deno_core::OpState>>,
	args: TargetArgs,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server
			.add_expression(&Expression::Target(crate::expression::Target {
				lockfile: args.lockfile,
				package: args.package,
				name: args.name,
				args: args.args,
			}))
			.await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_array(
	state: Rc<RefCell<deno_core::OpState>>,
	value: Vec<Hash>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_expression(&Expression::Array(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_map(
	state: Rc<RefCell<deno_core::OpState>>,
	value: BTreeMap<String, Hash>,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		let value = value
			.into_iter()
			.map(|(key, value)| (key.into(), value))
			.collect();
		server.add_expression(&Expression::Map(value)).await
	})
	.await
}

#[deno_core::op]
#[allow(clippy::unnecessary_wraps)]
async fn op_tangram_blob(
	state: Rc<RefCell<deno_core::OpState>>,
	value: String,
) -> Result<Hash, deno_core::error::AnyError> {
	op(state, |server| async move {
		server.add_blob(value.as_bytes()).await
	})
	.await
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
