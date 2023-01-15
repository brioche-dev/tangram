use super::{FutureOutput, State, THREAD_LOCAL_ISOLATE};
use crate::{
	artifact::{Artifact, ArtifactHash},
	blob::BlobHash,
	compiler::ModuleIdentifier,
	operation::Operation,
	package::{Package, PackageHash},
	value::Value,
	Cli,
};
use anyhow::{bail, Context, Result};
use itertools::Itertools;
use num::ToPrimitive;
use std::{future::Future, rc::Rc};

#[allow(clippy::needless_pass_by_value)]
pub fn syscall(
	scope: &mut v8::HandleScope,
	args: v8::FunctionCallbackArguments,
	mut return_value: v8::ReturnValue,
) {
	match syscall_inner(scope, &args) {
		Ok(value) => {
			return_value.set(value);
		},
		Err(error) => {
			let error = v8::String::new(scope, &error.to_string()).unwrap();
			scope.throw_exception(error.into());
		},
	}
}

fn syscall_inner<'s>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
) -> Result<v8::Local<'s, v8::Value>> {
	// Get the syscall name.
	let name: String =
		serde_v8::from_v8(scope, args.get(0)).context("Failed to deserialize the syscall name.")?;

	// Invoke the syscall.
	match name.as_str() {
		"print" => syscall_sync(scope, args, syscall_print),
		"serialize" => syscall_sync(scope, args, syscall_serialize),
		"deserialize" => syscall_sync(scope, args, syscall_deserialize),
		"add_blob" => syscall_async(scope, args, syscall_add_blob),
		"get_blob" => syscall_async(scope, args, syscall_get_blob),
		"add_artifact" => syscall_async(scope, args, syscall_add_artifact),
		"get_artifact" => syscall_async(scope, args, syscall_get_artifact),
		"add_package" => syscall_async(scope, args, syscall_add_package),
		"get_package" => syscall_async(scope, args, syscall_get_package),
		"run" => syscall_async(scope, args, syscall_run),
		"get_current_package_hash" => syscall_sync(scope, args, syscall_get_current_package_hash),
		"get_target_name" => syscall_sync(scope, args, syscall_get_target_name),
		_ => bail!(r#"Unknown syscall "{name}"."#),
	}
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_print(_scope: &mut v8::HandleScope, _state: Rc<State>, args: (String,)) -> Result<()> {
	let (string,) = args;
	println!("{string}");
	Ok(())
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
enum SerializationFormat {
	#[serde(rename = "toml")]
	Toml,
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_serialize(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (SerializationFormat, serde_json::Value),
) -> Result<String> {
	let (format, value) = args;
	match format {
		SerializationFormat::Toml => {
			let value = toml::to_string(&value)?;
			Ok(value)
		},
	}
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_deserialize(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (SerializationFormat, String),
) -> Result<serde_json::Value> {
	let (format, string) = args;
	match format {
		SerializationFormat::Toml => {
			let value = toml::from_str(&string)?;
			Ok(value)
		},
	}
}

async fn syscall_add_blob(cli: Cli, args: (serde_v8::ZeroCopyBuf,)) -> Result<BlobHash> {
	let (blob,) = args;
	let blob_hash = cli.add_blob(blob.as_ref()).await?;
	Ok(blob_hash)
}

async fn syscall_get_blob(cli: Cli, args: (BlobHash,)) -> Result<serde_v8::ZeroCopyBuf> {
	let (blob_hash,) = args;
	let mut blob = cli.get_blob(blob_hash).await?;
	let mut bytes = Vec::new();
	tokio::io::copy(&mut blob, &mut bytes).await?;
	let output = serde_v8::ZeroCopyBuf::ToV8(Some(bytes.into_boxed_slice()));
	Ok(output)
}

async fn syscall_add_artifact(cli: Cli, args: (Artifact,)) -> Result<ArtifactHash> {
	let (artifact,) = args;
	let artifact_hash = cli.add_artifact(&artifact).await?;
	Ok(artifact_hash)
}

#[allow(clippy::unused_async)]
async fn syscall_get_artifact(cli: Cli, args: (ArtifactHash,)) -> Result<Option<Artifact>> {
	let (artifact_hash,) = args;
	let artifact = cli.try_get_artifact_local(artifact_hash)?;
	Ok(artifact)
}

#[allow(clippy::unused_async)]
async fn syscall_add_package(cli: Cli, args: (Package,)) -> Result<PackageHash> {
	let (package,) = args;
	let package_hash = cli.add_package(&package)?;
	Ok(package_hash)
}

#[allow(clippy::unused_async)]
async fn syscall_get_package(cli: Cli, args: (PackageHash,)) -> Result<Option<Package>> {
	let (package_hash,) = args;
	let package = cli.try_get_package_local(package_hash)?;
	Ok(package)
}

async fn syscall_run(cli: Cli, args: (Operation,)) -> Result<Value> {
	let (operation,) = args;
	let output = cli.run(&operation).await?;
	Ok(output)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_get_current_package_hash(
	scope: &mut v8::HandleScope,
	_state: Rc<State>,
	_args: (),
) -> Result<PackageHash> {
	// Get the location.
	let Location {
		module_identifier: url,
		..
	} = get_location(scope);

	// Get the package hash.
	let package_hash = match url {
		ModuleIdentifier::Hash { package_hash, .. } => package_hash,
		_ => panic!(),
	};

	Ok(package_hash)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_get_target_name(
	scope: &mut v8::HandleScope,
	state: Rc<State>,
	_args: (),
) -> Result<String> {
	// Get the location.
	let Location {
		module_identifier: url,
		line,
		column,
	} = get_location(scope);

	// Get the module.
	let modules = state.modules.borrow();
	let module = modules
		.iter()
		.find(|module| module.module_identifier == url)
		.unwrap();

	// Apply a source map if one is available.
	let (line, _column) = module
		.source_map
		.as_ref()
		.map_or((line, column), |source_map| {
			let token = source_map.lookup_token(line, column).unwrap();
			let line = token.get_src_line();
			let column = token.get_src_col();
			(line, column)
		});

	// Get the caller's caller's source line.
	let line = module.source.lines().nth(line.to_usize().unwrap()).unwrap();

	// Get the target name.
	let name = if line.starts_with("export default") {
		"default".to_owned()
	} else if line.starts_with("export let") {
		line.split_whitespace().nth(2).unwrap().to_owned()
	} else {
		bail!("Invalid target.");
	};

	Ok(name)
}

struct Location {
	module_identifier: ModuleIdentifier,
	line: u32,
	column: u32,
}

fn get_location(scope: &mut v8::HandleScope) -> Location {
	// Get the module identifier, line, and column of the caller's caller.
	let stack_trace = v8::StackTrace::current_stack_trace(scope, 2).unwrap();
	let stack_frame = stack_trace.get_frame(scope, 1).unwrap();
	let module_identifier = stack_frame
		.get_script_name(scope)
		.unwrap()
		.to_rust_string_lossy(scope)
		.parse()
		.unwrap();
	let line = stack_frame.get_line_number().to_u32().unwrap() - 1;
	let column = stack_frame.get_column().to_u32().unwrap() - 1;
	Location {
		module_identifier,
		line,
		column,
	}
}

fn syscall_sync<'s, A, T, F>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(&mut v8::HandleScope<'s>, Rc<State>, A) -> Result<T>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the state.
	let state = Rc::clone(context.get_slot::<Rc<State>>(scope).unwrap());

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = serde_v8::from_v8(scope, args.into()).context("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(scope, state, args)?;

	// Serialize the value.
	let value = serde_v8::to_v8(scope, &value).context("Failed to serialize the value.")?;

	Ok(value)
}

#[allow(clippy::unnecessary_wraps)]
fn syscall_async<'s, A, T, F, Fut>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Cli, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the cli.
	let cli = context.get_slot::<Cli>(scope).unwrap().clone();

	// Get the state.
	let state = Rc::clone(context.get_slot::<Rc<State>>(scope).unwrap());

	// Create the promise.
	let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
	let value = promise_resolver.get_promise(scope);

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Move the promise resolver and args to the global scope.
	let context = v8::Global::new(scope, context);
	let promise_resolver = v8::Global::new(scope, promise_resolver);
	let args = v8::Global::new(scope, args);

	// Create the future.
	let future = Box::pin(async move {
		let result = syscall_async_inner(context.clone(), cli, args, f).await;
		FutureOutput {
			context,
			promise_resolver,
			result,
		}
	});

	// Add the future to the context's future set.
	state.futures.borrow_mut().push(future);

	Ok(value.into())
}

async fn syscall_async_inner<A, T, F, Fut>(
	context: v8::Global<v8::Context>,
	cli: Cli,
	args: v8::Global<v8::Array>,
	f: F,
) -> Result<v8::Global<v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Cli, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Deserialize the args.
	let args = {
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, &context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
		let args = v8::Local::new(&mut context_scope, args);
		serde_v8::from_v8(&mut context_scope, args.into())
			.context("Failed to deserialize the args.")?
	};

	// Call the function.
	let value = f(cli, args).await?;

	// Serialize the value.
	let value = {
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, &context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
		let value =
			serde_v8::to_v8(&mut context_scope, value).context("Failed to serialize the value.")?;
		v8::Global::new(&mut context_scope, value)
	};

	Ok(value)
}
