#![allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]

use super::{
	isolate::THREAD_LOCAL_ISOLATE,
	state::{FutureOutput, State},
};
use crate::{
	artifact::Artifact,
	blob::Blob,
	block::Block,
	checksum::{self, Checksum},
	directory::Directory,
	error::{return_error, Error, Result, WrapErr},
	file::File,
	instance::Instance,
	module::position::Position,
	module::Module,
	operation::Operation,
	path::Subpath,
	resource::{self, Resource},
	symlink::Symlink,
	system::System,
	target::Target,
	task::Task,
	template::Template,
	value::Value,
};
use base64::Engine as _;
use itertools::Itertools;
use std::{collections::BTreeMap, future::Future, rc::Rc};
use tracing::Instrument;
use url::Url;

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
		"blob_bytes" => syscall_async(scope, args, syscall_blob_bytes),
		"blob_get" => syscall_async(scope, args, syscall_blob_get),
		"blob_new" => syscall_async(scope, args, syscall_blob_new),
		"blob_text" => syscall_async(scope, args, syscall_blob_text),
		"block_bytes" => syscall_async(scope, args, syscall_block_bytes),
		"block_children" => syscall_async(scope, args, syscall_block_children),
		"block_data" => syscall_async(scope, args, syscall_block_data),
		"block_new" => syscall_async(scope, args, syscall_block_new),
		"checksum" => syscall_sync(scope, args, syscall_checksum),
		"directory_new" => syscall_async(scope, args, syscall_directory_new),
		"encoding_base64_decode" => syscall_sync(scope, args, syscall_encoding_base64_decode),
		"encoding_base64_encode" => syscall_sync(scope, args, syscall_encoding_base64_encode),
		"encoding_hex_decode" => syscall_sync(scope, args, syscall_encoding_hex_decode),
		"encoding_hex_encode" => syscall_sync(scope, args, syscall_encoding_hex_encode),
		"encoding_json_decode" => syscall_sync(scope, args, syscall_encoding_json_decode),
		"encoding_json_encode" => syscall_sync(scope, args, syscall_encoding_json_encode),
		"encoding_toml_decode" => syscall_sync(scope, args, syscall_encoding_toml_decode),
		"encoding_toml_encode" => syscall_sync(scope, args, syscall_encoding_toml_encode),
		"encoding_utf8_decode" => syscall_sync(scope, args, syscall_encoding_utf8_decode),
		"encoding_utf8_encode" => syscall_sync(scope, args, syscall_encoding_utf8_encode),
		"encoding_yaml_decode" => syscall_sync(scope, args, syscall_encoding_yaml_decode),
		"encoding_yaml_encode" => syscall_sync(scope, args, syscall_encoding_yaml_encode),
		"file_new" => syscall_async(scope, args, syscall_file_new),
		"log" => syscall_sync(scope, args, syscall_log),
		"operation_evaluate" => syscall_async(scope, args, syscall_operation_evaluate),
		"operation_get" => syscall_async(scope, args, syscall_operation_get),
		"resource_new" => syscall_async(scope, args, syscall_resource_new),
		"symlink_new" => syscall_async(scope, args, syscall_symlink_new),
		"target_new" => syscall_async(scope, args, syscall_target_new),
		"task_new" => syscall_async(scope, args, syscall_task_new),
		_ => return_error!(r#"Unknown syscall "{name}"."#),
	}
}

async fn syscall_artifact_bundle(tg: Instance, args: (Artifact,)) -> Result<Artifact> {
	let (artifact,) = args;
	let artifact = artifact.bundle(&tg).await?;
	Ok(artifact)
}

async fn syscall_artifact_get(tg: Instance, args: (Block,)) -> Result<Artifact> {
	let (block,) = args;
	let artifact = Artifact::get(&tg, block).await?;
	Ok(artifact)
}

async fn syscall_blob_bytes(tg: Instance, args: (Blob,)) -> Result<serde_v8::ToJsBuffer> {
	let (blob,) = args;
	let bytes = blob.bytes(&tg).await?;
	Ok(bytes.into())
}

async fn syscall_blob_get(tg: Instance, args: (Block,)) -> Result<Blob> {
	let (block,) = args;
	let blob = Blob::get(&tg, block).await?;
	Ok(blob)
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlobArg {
	children: Vec<Block>,
}

async fn syscall_blob_new(tg: Instance, args: (BlobArg,)) -> Result<Blob> {
	let (BlobArg { children },) = args;
	let blob = Blob::new(&tg, children).await?;
	Ok(blob)
}

async fn syscall_blob_text(tg: Instance, args: (Blob,)) -> Result<String> {
	let (blob,) = args;
	let text = blob.text(&tg).await?;
	Ok(text)
}

async fn syscall_block_bytes(tg: Instance, args: (Block,)) -> Result<serde_v8::ToJsBuffer> {
	let (block,) = args;
	let bytes = block.bytes(&tg).await?;
	Ok(bytes.into())
}

async fn syscall_block_data(tg: Instance, args: (Block,)) -> Result<serde_v8::ToJsBuffer> {
	let (block,) = args;
	let data = block.data(&tg).await?;
	Ok(data.into())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockArg {
	children: Vec<Block>,
	data: serde_v8::StringOrBuffer,
}

async fn syscall_block_new(tg: Instance, args: (BlockArg,)) -> Result<Block> {
	let (BlockArg { data, children },) = args;
	let block = Block::new(&tg, children, &data).await?;
	Ok(block)
}

async fn syscall_block_children(tg: Instance, args: (Block,)) -> Result<Vec<Block>> {
	let (block,) = args;
	let references = block.children(&tg).await?;
	Ok(references)
}

fn syscall_checksum(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (checksum::Algorithm, serde_v8::JsBuffer),
) -> Result<Checksum> {
	let (algorithm, bytes) = args;
	let mut checksum_writer = checksum::Writer::new(algorithm);
	checksum_writer.update(&bytes);
	let checksum = checksum_writer.finalize();
	Ok(checksum)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryArg {
	entries: BTreeMap<String, Artifact>,
}

async fn syscall_directory_new(tg: Instance, args: (DirectoryArg,)) -> Result<Directory> {
	let (arg,) = args;
	let directory = Directory::new(&tg, &arg.entries).await?;
	Ok(directory)
}

fn syscall_encoding_base64_decode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (String,),
) -> Result<serde_v8::ToJsBuffer> {
	let (value,) = args;
	let bytes = base64::engine::general_purpose::STANDARD_NO_PAD
		.decode(value)
		.map_err(Error::other)
		.wrap_err("Failed to decode the bytes.")?;
	Ok(bytes.into())
}

fn syscall_encoding_base64_encode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (serde_v8::JsBuffer,),
) -> Result<String> {
	let (value,) = args;
	let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(value);
	Ok(encoded)
}

fn syscall_encoding_hex_decode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (serde_v8::JsBuffer,),
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

fn syscall_encoding_hex_encode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (String,),
) -> Result<serde_v8::ToJsBuffer> {
	let (bytes,) = args;
	let hex = hex::encode(bytes);
	let bytes = hex.into_bytes().into();
	Ok(bytes)
}

fn syscall_encoding_json_decode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (String,),
) -> Result<serde_json::Value> {
	let (json,) = args;
	let value = serde_json::from_str(&json)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as json.")?;
	Ok(value)
}

fn syscall_encoding_json_encode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (serde_json::Value,),
) -> Result<String> {
	let (value,) = args;
	let json = serde_json::to_string(&value)
		.map_err(Error::other)
		.wrap_err("Failed to encode the value.")?;
	Ok(json)
}

fn syscall_encoding_toml_decode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (String,),
) -> Result<toml::Value> {
	let (toml,) = args;
	let value = toml::from_str(&toml)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as toml.")?;
	Ok(value)
}

fn syscall_encoding_toml_encode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (toml::Value,),
) -> Result<String> {
	let (value,) = args;
	let toml = toml::to_string(&value)
		.map_err(Error::other)
		.wrap_err("Failed to encode the value.")?;
	Ok(toml)
}

fn syscall_encoding_utf8_decode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (serde_v8::JsBuffer,),
) -> Result<String> {
	let (bytes,) = args;
	let bytes = bytes::Bytes::from(bytes);
	let string = String::from_utf8(bytes.into())
		.map_err(Error::other)
		.wrap_err("Failed to decode the bytes as UTF-8.")?;
	Ok(string)
}

fn syscall_encoding_utf8_encode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (String,),
) -> Result<serde_v8::ToJsBuffer> {
	let (string,) = args;
	let bytes = string.into_bytes().into();
	Ok(bytes)
}

fn syscall_encoding_yaml_decode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (String,),
) -> Result<serde_yaml::Value> {
	let (yaml,) = args;
	let value = serde_yaml::from_str(&yaml)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as yaml.")?;
	Ok(value)
}

fn syscall_encoding_yaml_encode(
	_scope: &mut v8::HandleScope,
	_tg: Instance,
	args: (serde_yaml::Value,),
) -> Result<String> {
	let (value,) = args;
	let yaml = serde_yaml::to_string(&value)
		.map_err(Error::other)
		.wrap_err("Failed to encode the value.")?;
	Ok(yaml)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileArg {
	contents: Blob,
	executable: bool,
	references: Vec<Artifact>,
}

async fn syscall_file_new(tg: Instance, args: (FileArg,)) -> Result<File> {
	let (arg,) = args;
	let file = File::new(&tg, &arg.contents, arg.executable, &arg.references).await?;
	Ok(file)
}

fn syscall_log(_scope: &mut v8::HandleScope, _tg: Instance, args: (String,)) -> Result<()> {
	let (string,) = args;
	println!("{string}");
	Ok(())
}

async fn syscall_operation_evaluate(tg: Instance, args: (Operation,)) -> Result<Value> {
	let (operation,) = args;
	let value = operation.evaluate(&tg, None).await?;
	Ok(value)
}

async fn syscall_operation_get(tg: Instance, args: (Block,)) -> Result<Operation> {
	let (block,) = args;
	let operation = Operation::get(&tg, block).await?;
	Ok(operation)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceArg {
	url: Url,
	unpack: Option<resource::unpack::Format>,
	checksum: Option<Checksum>,
	unsafe_: bool,
}

async fn syscall_resource_new(tg: Instance, args: (ResourceArg,)) -> Result<Resource> {
	let (arg,) = args;
	let download = Resource::new(&tg, arg.url, arg.unpack, arg.checksum, arg.unsafe_).await?;
	Ok(download)
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

async fn syscall_symlink_new(tg: Instance, args: (SymlinkArg,)) -> Result<Symlink> {
	let (arg,) = args;
	let symlink = Symlink::new(&tg, arg.target).await?;
	Ok(symlink)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TargetArg {
	package: Block,
	module_path: Subpath,
	name: String,
	env: BTreeMap<String, Value>,
	args: Vec<Value>,
}

async fn syscall_target_new(tg: Instance, args: (TargetArg,)) -> Result<Target> {
	let (arg,) = args;
	let target = Target::new(
		&tg,
		arg.package,
		arg.module_path,
		arg.name,
		arg.env,
		arg.args,
	)
	.await?;
	Ok(target)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskArg {
	system: System,
	executable: Template,
	env: BTreeMap<String, Template>,
	args: Vec<Template>,
	checksum: Option<Checksum>,
	unsafe_: bool,
	network: bool,
}

async fn syscall_task_new(tg: Instance, args: (TaskArg,)) -> Result<Task> {
	let (arg,) = args;
	let task = Task::builder(arg.system, arg.executable)
		.env(arg.env)
		.args(arg.args)
		.checksum(arg.checksum)
		.unsafe_(arg.unsafe_)
		.network(arg.network)
		.build(&tg)
		.await?;
	Ok(task)
}

fn syscall_sync<'s, A, T, F>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(&mut v8::HandleScope<'s>, Instance, A) -> Result<T>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the instance.
	let tg = context.get_slot::<Instance>(scope).unwrap().clone();

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = serde_v8::from_v8(scope, args.into())
		.map_err(Error::other)
		.wrap_err("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(scope, tg, args)?;

	// Serialize the value.
	let value = serde_v8::to_v8(scope, &value)
		.map_err(Error::other)
		.wrap_err("Failed to serialize the value.")?;

	Ok(value)
}

fn syscall_async<'s, A, T, F, Fut>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Instance, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the instance.
	let tg = context.get_slot::<Instance>(scope).unwrap().clone();

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
	tg: Instance,
	args: v8::Global<v8::Array>,
	f: F,
) -> Result<v8::Global<v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Instance, A) -> Fut,
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
