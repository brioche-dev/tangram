use super::{ContextState, FutureOutput, THREAD_LOCAL_ISOLATE};
use crate::{
	artifact::{Artifact, ArtifactHash},
	blob::BlobHash,
	compiler,
	operation::Operation,
	package::PackageHash,
	value::Value,
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

#[allow(clippy::too_many_lines)]
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
		"run" => syscall_async(scope, args, syscall_run),
		"get_target_info" => syscall_sync(scope, args, syscall_get_target_info),
		_ => {
			bail!(r#"Unknown syscall "{name}"."#);
		},
	}
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_print(
	_scope: &mut v8::HandleScope,
	_state: Rc<ContextState>,
	args: (String,),
) -> Result<()> {
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
	_state: Rc<ContextState>,
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
	_state: Rc<ContextState>,
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

async fn syscall_add_blob(
	state: Rc<ContextState>,
	args: (serde_v8::ZeroCopyBuf,),
) -> Result<BlobHash> {
	let (blob,) = args;
	let cli = state.cli.clone();
	let cli = cli.lock_shared().await?;
	let blob_hash = cli.add_blob(blob.as_ref()).await?;
	Ok(blob_hash)
}

async fn syscall_get_blob(
	state: Rc<ContextState>,
	args: (BlobHash,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (blob_hash,) = args;
	let cli = state.cli.clone();
	let cli = cli.lock_shared().await?;
	let mut blob = cli.get_blob(blob_hash).await?;
	let mut bytes = Vec::new();
	tokio::io::copy(&mut blob, &mut bytes).await?;
	let output = serde_v8::ZeroCopyBuf::ToV8(Some(bytes.into_boxed_slice()));
	Ok(output)
}

async fn syscall_add_artifact(state: Rc<ContextState>, args: (Artifact,)) -> Result<ArtifactHash> {
	let (artifact,) = args;
	let cli = state.cli.clone();
	let cli = cli.lock_shared().await?;
	let artifact_hash = cli.add_artifact(&artifact).await?;
	Ok(artifact_hash)
}

async fn syscall_get_artifact(
	state: Rc<ContextState>,
	args: (ArtifactHash,),
) -> Result<Option<Artifact>> {
	let (hash,) = args;
	let cli = state.cli.clone();
	let cli = cli.lock_shared().await?;
	let artifact = cli.try_get_artifact_local(hash)?;
	Ok(artifact)
}

async fn syscall_run(state: Rc<ContextState>, args: (Operation,)) -> Result<Value> {
	let (operation,) = args;
	let cli = state.cli.clone();
	let cli = cli.lock_shared().await?;
	let output = cli.run(&operation).await?;
	Ok(output)
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
	pub package_hash: PackageHash,
	pub name: String,
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_get_target_info(
	scope: &mut v8::HandleScope,
	state: Rc<ContextState>,
	_args: (),
) -> Result<TargetInfo> {
	// Get the URL, line, and column of the caller's caller.
	let stack_trace = v8::StackTrace::current_stack_trace(scope, 2).unwrap();
	let stack_frame = stack_trace.get_frame(scope, 1).unwrap();
	let url = stack_frame
		.get_script_name(scope)
		.unwrap()
		.to_rust_string_lossy(scope);
	let url: compiler::Url = url.parse().unwrap();
	let line = stack_frame.get_line_number().to_u32().unwrap() - 1;
	let column = stack_frame.get_column().to_u32().unwrap() - 1;

	// Get the package hash.
	let package_hash = match url {
		compiler::Url::Hash { package_hash, .. } => package_hash,
		_ => panic!(),
	};

	// Get the module.
	let modules = state.modules.borrow();
	let module = modules.iter().find(|module| module.url == url).unwrap();

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

	Ok(TargetInfo { package_hash, name })
}

fn syscall_sync<'s, A, T, F>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(&mut v8::HandleScope<'s>, Rc<ContextState>, A) -> Result<T>,
{
	// Retrieve the context and context state.
	let context = scope.get_current_context();
	let context_state = Rc::clone(context.get_slot::<Rc<ContextState>>(scope).unwrap());

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = serde_v8::from_v8(scope, args.into()).context("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(scope, context_state, args)?;

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
	F: FnOnce(Rc<ContextState>, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Retrieve the isolate state, context, and context state.
	let context = scope.get_current_context();
	let context_state = Rc::clone(context.get_slot::<Rc<ContextState>>(scope).unwrap());

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
	let future = Box::pin({
		let context_state = Rc::clone(&context_state);
		async move {
			let result = syscall_async_inner(context.clone(), context_state, args, f).await;
			FutureOutput {
				context,
				promise_resolver,
				result,
			}
		}
	});

	// Add the future to the context's future set.
	context_state.futures.borrow_mut().push(future);

	Ok(value.into())
}

async fn syscall_async_inner<A, T, F, Fut>(
	context: v8::Global<v8::Context>,
	context_state: Rc<ContextState>,
	args: v8::Global<v8::Array>,
	f: F,
) -> Result<v8::Global<v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Rc<ContextState>, A) -> Fut + 'static,
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
	let value = f(context_state, args).await?;

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
