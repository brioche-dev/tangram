use super::{
	isolate::THREAD_LOCAL_ISOLATE,
	state::{FutureOutput, State},
};
use crate::{
	artifact::{self, Artifact},
	blob::Blob,
	checksum::{self, Checksum},
	command::Command,
	directory::Directory,
	error::{return_error, Error, Result, WrapErr},
	file::File,
	function::Function,
	instance::Instance,
	module::position::Position,
	module::Module,
	operation::{self, Operation},
	package::{self, Package},
	path::Subpath,
	resource::Resource,
	symlink::Symlink,
	system::System,
	template::Template,
	value::Value,
};
use base64::Engine as _;
use itertools::Itertools;
use std::{collections::BTreeMap, future::Future, rc::Rc, sync::Arc};
use tracing::Instrument;
use url::Url;

#[allow(clippy::needless_pass_by_value)]
pub fn syscall(
	scope: &mut v8::HandleScope,
	args: v8::FunctionCallbackArguments,
	mut return_value: v8::ReturnValue,
) {
	match syscall_inner(scope, &args) {
		Ok(value) => {
			// Set the return value.
			return_value.set(value);
		},

		Err(error) => {
			// Throw the exception.
			let exception = error.to_exception(scope);
			scope.throw_exception(exception);
		},
	}
}

fn syscall_inner<'s>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
) -> Result<v8::Local<'s, v8::Value>> {
	// Get the syscall name.
	let name: String = serde_v8::from_v8(scope, args.get(0))
		.map_err(Error::other)
		.wrap_err("Failed to deserialize the syscall name.")?;

	// Invoke the syscall.
	match name.as_str() {
		"artifact_bundle" => syscall_async(scope, args, syscall_artifact_bundle),
		"artifact_get" => syscall_async(scope, args, syscall_artifact_get),
		"base64_decode" => syscall_sync(scope, args, syscall_base64_decode),
		"base64_encode" => syscall_sync(scope, args, syscall_base64_encode),
		"blob_bytes" => syscall_async(scope, args, syscall_blob_bytes),
		"blob_new" => syscall_async(scope, args, syscall_blob_new),
		"blob_text" => syscall_async(scope, args, syscall_blob_text),
		"checksum" => syscall_sync(scope, args, syscall_checksum),
		"command_new" => syscall_async(scope, args, syscall_command_new),
		"directory_new" => syscall_async(scope, args, syscall_directory_new),
		"file_new" => syscall_async(scope, args, syscall_file_new),
		"function_new" => syscall_async(scope, args, syscall_function_new),
		"hex_decode" => syscall_sync(scope, args, syscall_hex_decode),
		"hex_encode" => syscall_sync(scope, args, syscall_hex_encode),
		"json_decode" => syscall_sync(scope, args, syscall_json_decode),
		"json_encode" => syscall_sync(scope, args, syscall_json_encode),
		"log" => syscall_sync(scope, args, syscall_log),
		"operation_get" => syscall_async(scope, args, syscall_operation_get),
		"operation_run" => syscall_async(scope, args, syscall_operation_run),
		"resource_new" => syscall_async(scope, args, syscall_resource_new),
		"symlink_new" => syscall_async(scope, args, syscall_symlink_new),
		"toml_decode" => syscall_sync(scope, args, syscall_toml_decode),
		"toml_encode" => syscall_sync(scope, args, syscall_toml_encode),
		"utf8_decode" => syscall_sync(scope, args, syscall_utf8_decode),
		"utf8_encode" => syscall_sync(scope, args, syscall_utf8_encode),
		"yaml_decode" => syscall_sync(scope, args, syscall_yaml_decode),
		"yaml_encode" => syscall_sync(scope, args, syscall_yaml_encode),
		_ => return_error!(r#"Unknown syscall "{name}"."#),
	}
}

async fn syscall_artifact_bundle(tg: Arc<Instance>, args: (Artifact,)) -> Result<Artifact> {
	let (artifact,) = args;
	let artifact = artifact.bundle(&tg).await?;
	Ok(artifact)
}

async fn syscall_artifact_get(tg: Arc<Instance>, args: (artifact::Hash,)) -> Result<Artifact> {
	let (hash,) = args;
	let artifact = Artifact::get(&tg, hash).await?;
	Ok(artifact)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_base64_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (value,) = args;
	let bytes = base64::engine::general_purpose::STANDARD_NO_PAD
		.decode(value)
		.map_err(Error::other)
		.wrap_err("Failed to decode the bytes.")?;
	Ok(bytes.into())
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_base64_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_v8::ZeroCopyBuf,),
) -> Result<String> {
	let (value,) = args;
	let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(value);
	Ok(encoded)
}

async fn syscall_blob_bytes(tg: Arc<Instance>, args: (Blob,)) -> Result<serde_v8::ZeroCopyBuf> {
	let (blob,) = args;
	let bytes = blob.bytes(&tg).await?;
	Ok(bytes.into())
}

async fn syscall_blob_new(tg: Arc<Instance>, args: (serde_v8::StringOrBuffer,)) -> Result<Blob> {
	let (bytes,) = args;
	let bytes = match &bytes {
		serde_v8::StringOrBuffer::String(string) => string.as_bytes(),
		serde_v8::StringOrBuffer::Buffer(buffer) => buffer.as_ref(),
	};
	let blob = Blob::new(&tg, bytes).await?;
	Ok(blob)
}

async fn syscall_blob_text(tg: Arc<Instance>, args: (Blob,)) -> Result<String> {
	let (blob,) = args;
	let text = blob.text(&tg).await?;
	Ok(text)
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

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommandArg {
	system: System,
	executable: Template,
	env: Option<BTreeMap<String, Template>>,
	args: Option<Vec<Template>>,
	checksum: Option<Checksum>,
	unsafe_: Option<bool>,
	network: Option<bool>,
	host_paths: Option<Vec<String>>,
}

async fn syscall_command_new(tg: Arc<Instance>, args: (CommandArg,)) -> Result<Command> {
	let (arg,) = args;
	let mut command = Command::builder(arg.system, arg.executable);
	if let Some(env) = arg.env {
		command = command.env(env);
	}
	if let Some(args) = arg.args {
		command = command.args(args);
	}
	if let Some(checksum) = arg.checksum {
		command = command.checksum(checksum);
	}
	if let Some(unsafe_) = arg.unsafe_ {
		command = command.unsafe_(unsafe_);
	}
	if let Some(network) = arg.network {
		command = command.network(network);
	}
	if let Some(host_paths) = arg.host_paths {
		command = command.host_paths(host_paths);
	}
	let command = command.build(&tg).await?;
	Ok(command)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryArg {
	entries: BTreeMap<String, Artifact>,
}

async fn syscall_directory_new(tg: Arc<Instance>, args: (DirectoryArg,)) -> Result<Directory> {
	let (arg,) = args;
	let directory = Directory::new(&tg, arg.entries).await?;
	Ok(directory)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceArg {
	url: Url,
	unpack: bool,
	checksum: Option<Checksum>,
	unsafe_: bool,
}

async fn syscall_resource_new(tg: Arc<Instance>, args: (ResourceArg,)) -> Result<Resource> {
	let (arg,) = args;
	let download = Resource::new(&tg, arg.url, arg.unpack, arg.checksum, arg.unsafe_).await?;
	Ok(download)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileArg {
	blob: Blob,
	executable: bool,
	references: Vec<Artifact>,
}

async fn syscall_file_new(tg: Arc<Instance>, args: (FileArg,)) -> Result<File> {
	let (arg,) = args;
	let file = File::new(&tg, arg.blob, arg.executable, &arg.references).await?;
	Ok(file)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FunctionArg {
	package_hash: package::Hash,
	module_path: Subpath,
	name: String,
	env: BTreeMap<String, Value>,
	args: Vec<Value>,
}

async fn syscall_function_new(tg: Arc<Instance>, args: (FunctionArg,)) -> Result<Function> {
	let (arg,) = args;
	let package = Package::get(&tg, arg.package_hash).await?;
	let function =
		Function::new(&tg, package, arg.module_path, arg.name, arg.env, arg.args).await?;
	Ok(function)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_hex_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_v8::ZeroCopyBuf,),
) -> Result<String> {
	let (hex,) = args;
	let bytes = hex::decode(hex)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as hex.")?;
	let string = String::from_utf8(bytes)
		.map_err(Error::other)
		.wrap_err("Failed to decode the bytes as UTF-8.")?;
	Ok(string)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_hex_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (bytes,) = args;
	let hex = hex::encode(bytes);
	let bytes = hex.into_bytes().into();
	Ok(bytes)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_json_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_json::Value> {
	let (json,) = args;
	let value = serde_json::from_str(&json)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as json.")?;
	Ok(value)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_json_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_json::Value,),
) -> Result<String> {
	let (value,) = args;
	let json = serde_json::to_string(&value)
		.map_err(Error::other)
		.wrap_err("Failed to encode the value.")?;
	Ok(json)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_log(_scope: &mut v8::HandleScope, _state: Rc<State>, args: (String,)) -> Result<()> {
	let (string,) = args;
	println!("{string}");
	Ok(())
}

async fn syscall_operation_get(tg: Arc<Instance>, args: (operation::Hash,)) -> Result<Operation> {
	let (hash,) = args;
	let operation = Operation::get(&tg, hash).await?;
	Ok(operation)
}

async fn syscall_operation_run(tg: Arc<Instance>, args: (Operation,)) -> Result<Value> {
	let (operation,) = args;
	// TODO: Set the parent operation here.
	let value = operation.run(&tg).await?;
	Ok(value)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StackFrame {
	module: Module,
	position: Position,
	line: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymlinkArg {
	target: Template,
}

async fn syscall_symlink_new(tg: Arc<Instance>, args: (SymlinkArg,)) -> Result<Symlink> {
	let (arg,) = args;
	let symlink = Symlink::new(&tg, arg.target).await?;
	Ok(symlink)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_toml_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<toml::Value> {
	let (toml,) = args;
	let value = toml::from_str(&toml)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as toml.")?;
	Ok(value)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_toml_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (toml::Value,),
) -> Result<String> {
	let (value,) = args;
	let toml = toml::to_string(&value)
		.map_err(Error::other)
		.wrap_err("Failed to encode the value.")?;
	Ok(toml)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_utf8_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_v8::ZeroCopyBuf,),
) -> Result<String> {
	let (bytes,) = args;
	let bytes = bytes::Bytes::from(bytes);
	let string = String::from_utf8(bytes.into())
		.map_err(Error::other)
		.wrap_err("Failed to decode the bytes as UTF-8.")?;
	Ok(string)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_utf8_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (string,) = args;
	let bytes = string.into_bytes().into();
	Ok(bytes)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_yaml_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_yaml::Value> {
	let (yaml,) = args;
	let value = serde_yaml::from_str(&yaml)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as yaml.")?;
	Ok(value)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_yaml_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_yaml::Value,),
) -> Result<String> {
	let (value,) = args;
	let yaml = serde_yaml::to_string(&value)
		.map_err(Error::other)
		.wrap_err("Failed to encode the value.")?;
	Ok(yaml)
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
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = serde_v8::from_v8(scope, args.into())
		.map_err(Error::other)
		.wrap_err("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(scope, state, args)?;

	// Serialize the value.
	let value = serde_v8::to_v8(scope, &value)
		.map_err(Error::other)
		.wrap_err("Failed to serialize the value.")?;

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
	let tg = context.get_slot::<Arc<Instance>>(scope).unwrap().clone();

	// Get the state.
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();

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
	let future = async move {
		let result = syscall_async_inner(context.clone(), tg, args, f).await;
		FutureOutput {
			context,
			promise_resolver,
			result,
		}
	};
	let future = future.instrument(tracing::info_span!("syscall_async"));
	let future = Box::pin(future);

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
			.map_err(Error::other)
			.wrap_err("Failed to deserialize the args.")?
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
		let value = serde_v8::to_v8(&mut context_scope, value)
			.map_err(Error::other)
			.wrap_err("Failed to serialize the value.")?;
		v8::Global::new(&mut context_scope, value)
	};

	Ok(value)
}
