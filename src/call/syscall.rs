use super::{
	context::{FutureOutput, State},
	isolate::THREAD_LOCAL_ISOLATE,
};
use crate::{
	artifact::{self, Artifact},
	blob,
	checksum::{self, Checksum},
	error::{bail, Context, Result},
	language::Position,
	module,
	operation::Operation,
	package,
	value::Value,
	Instance,
};
use itertools::Itertools;
use num::ToPrimitive;
use std::{future::Future, rc::Rc, sync::Arc};

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
		"log" => syscall_sync(scope, args, syscall_log),
		"checksum" => syscall_sync(scope, args, syscall_checksum),
		"encode_utf8" => syscall_sync(scope, args, syscall_encode_utf8),
		"decode_utf8" => syscall_sync(scope, args, syscall_decode_utf8),
		"add_blob" => syscall_async(scope, args, syscall_add_blob),
		"get_blob" => syscall_async(scope, args, syscall_get_blob),
		"add_artifact" => syscall_async(scope, args, syscall_add_artifact),
		"get_artifact" => syscall_async(scope, args, syscall_get_artifact),
		"add_package_instance" => syscall_async(scope, args, syscall_add_package_instance),
		"get_package_instance" => syscall_async(scope, args, syscall_get_package_instance),
		"run" => syscall_async(scope, args, syscall_run),
		"get_current_package_instance_hash" => {
			syscall_sync(scope, args, syscall_get_current_package_instance_hash)
		},
		"get_current_export_name" => syscall_sync(scope, args, syscall_get_current_export_name),
		_ => bail!(r#"Unknown syscall "{name}"."#),
	}
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_log(_scope: &mut v8::HandleScope, _state: Rc<State>, args: (String,)) -> Result<()> {
	let (string,) = args;
	println!("{string}");
	Ok(())
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_checksum(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (checksum::Algorithm, serde_v8::ZeroCopyBuf),
) -> Result<Checksum> {
	let (algorithm, bytes) = args;
	let mut checksum_writer = checksum::Writer::new(algorithm);
	checksum_writer.update(&bytes);
	let checksum = checksum_writer.finalize();
	Ok(checksum)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_encode_utf8(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (string,) = args;
	let bytes = string.into_bytes().into();
	Ok(bytes)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_decode_utf8(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_v8::ZeroCopyBuf,),
) -> Result<String> {
	let (bytes,) = args;
	let bytes = bytes::Bytes::from(bytes);
	let string = String::from_utf8(bytes.into()).context("Failed to decode the bytes as UTF-8.")?;
	Ok(string)
}

async fn syscall_add_blob(tg: Arc<Instance>, args: (serde_v8::ZeroCopyBuf,)) -> Result<blob::Hash> {
	let (blob,) = args;
	let blob_hash = tg.add_blob(blob.as_ref()).await?;
	Ok(blob_hash)
}

async fn syscall_get_blob(tg: Arc<Instance>, args: (blob::Hash,)) -> Result<serde_v8::ZeroCopyBuf> {
	let (blob_hash,) = args;
	let mut blob = tg.get_blob(blob_hash).await?;
	let mut bytes = Vec::new();
	tokio::io::copy(&mut blob, &mut bytes).await?;
	let output = serde_v8::ZeroCopyBuf::ToV8(Some(bytes.into_boxed_slice()));
	Ok(output)
}

async fn syscall_add_artifact(tg: Arc<Instance>, args: (Artifact,)) -> Result<artifact::Hash> {
	let (artifact,) = args;
	let artifact_hash = tg.add_artifact(&artifact).await?;
	Ok(artifact_hash)
}

#[allow(clippy::unused_async)]
async fn syscall_get_artifact(
	tg: Arc<Instance>,
	args: (artifact::Hash,),
) -> Result<Option<Artifact>> {
	let (artifact_hash,) = args;
	let artifact = tg.try_get_artifact_local(artifact_hash)?;
	Ok(artifact)
}

#[allow(clippy::unused_async)]
async fn syscall_add_package_instance(
	tg: Arc<Instance>,
	args: (package::instance::Instance,),
) -> Result<package::instance::Hash> {
	let (package,) = args;
	let package_instance_hash = tg.add_package_instance(&package)?;
	Ok(package_instance_hash)
}

#[allow(clippy::unused_async)]
async fn syscall_get_package_instance(
	tg: Arc<Instance>,
	args: (package::instance::Hash,),
) -> Result<Option<package::instance::Instance>> {
	let (package_instance_hash,) = args;
	let package = tg.try_get_package_instance_local(package_instance_hash)?;
	Ok(package)
}

async fn syscall_run(tg: Arc<Instance>, args: (Operation,)) -> Result<Value> {
	let (operation,) = args;
	let operation_hash = tg.add_operation(&operation)?;
	let output = tg.run(operation_hash).await?;
	Ok(output)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_get_current_package_instance_hash(
	scope: &mut v8::HandleScope,
	_state: Rc<State>,
	_args: (),
) -> Result<package::instance::Hash> {
	// Get the location.
	let (module_identifier, _) = get_module_identifier_and_position(scope);

	// Get the package instance hash.
	let module::Identifier::Normal(module::identifier::Normal { source : module::identifier::Source::Instance(package_instance_hash), .. }) = module_identifier else {
		bail!("The module identifier must be a normal module whose source is a package instance.");
	};

	Ok(package_instance_hash)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_get_current_export_name(
	scope: &mut v8::HandleScope,
	state: Rc<State>,
	_args: (),
) -> Result<String> {
	// Get the location.
	let (module_identifier, position) = get_module_identifier_and_position(scope);

	// Get the module.
	let modules = state.modules.borrow();
	let module = modules
		.iter()
		.find(|module| module.module_identifier == module_identifier)
		.unwrap();

	// Apply a source map if one is available.
	let position = module.source_map.as_ref().map_or(position, |source_map| {
		let token = source_map
			.lookup_token(position.line, position.character)
			.unwrap();
		let line = token.get_src_line();
		let character = token.get_src_col();
		Position { line, character }
	});

	// Get the caller's caller's source line.
	let line = module
		.text
		.lines()
		.nth(position.line.to_usize().unwrap())
		.unwrap();

	// Get the name.
	let name = if line.starts_with("export default") {
		"default".to_owned()
	} else if line.starts_with("export let") {
		line.split_whitespace().nth(2).unwrap().to_owned()
	} else {
		bail!("Invalid usage of tg.function.");
	};

	Ok(name)
}

fn get_module_identifier_and_position(
	scope: &mut v8::HandleScope,
) -> (module::Identifier, Position) {
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
	let character = stack_frame.get_column().to_u32().unwrap() - 1;
	let position = Position { line, character };
	(module_identifier, position)
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
	F: FnOnce(Arc<Instance>, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the instance.
	let tg = Arc::clone(context.get_slot::<Arc<Instance>>(scope).unwrap());

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
		let result = syscall_async_inner(context.clone(), tg, args, f).await;
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
	tg: Arc<Instance>,
	args: v8::Global<v8::Array>,
	f: F,
) -> Result<v8::Global<v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Arc<Instance>, A) -> Fut + 'static,
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
	let value = f(tg, args).await?;

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
