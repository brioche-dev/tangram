#![allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]

use super::{
	convert::{from_v8, FromV8, ToV8},
	FutureOutput, State, THREAD_LOCAL_ISOLATE,
};
use crate::{
	checksum, object, return_error, Artifact, Blob, Bytes, Checksum, Client, Error, Result, Task,
	Value, WrapErr,
};
use base64::Engine as _;
use itertools::Itertools;
use std::{future::Future, rc::Rc};
use url::Url;

pub fn syscall<'s>(
	scope: &mut v8::HandleScope<'s>,
	args: v8::FunctionCallbackArguments<'s>,
	mut return_value: v8::ReturnValue,
) {
	match syscall_inner(scope, &args) {
		Ok(value) => {
			// Set the return value.
			return_value.set(value);
		},

		Err(error) => {
			// Throw the exception.
			let exception = error.to_v8(scope).expect("Failed to serialize the error.");
			scope.throw_exception(exception);
		},
	}
}

fn syscall_inner<'s>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments<'s>,
) -> Result<v8::Local<'s, v8::Value>> {
	// Get the syscall name.
	let name =
		String::from_v8(scope, args.get(0)).wrap_err("Failed to deserialize the syscall name.")?;

	// Invoke the syscall.
	match name.as_str() {
		"bundle" => syscall_async(scope, args, syscall_bundle),
		"checksum" => syscall_sync(scope, args, syscall_checksum),
		"download" => syscall_async(scope, args, syscall_download),
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
		"load" => syscall_async(scope, args, syscall_load),
		"log" => syscall_sync(scope, args, syscall_log),
		"read" => syscall_async(scope, args, syscall_read),
		"run" => syscall_async(scope, args, syscall_run),
		"store" => syscall_async(scope, args, syscall_store),
		"unpack" => syscall_async(scope, args, syscall_unpack),
		_ => return_error!(r#"Unknown syscall "{name}"."#),
	}
}

async fn syscall_bundle(client: Client, args: (Artifact,)) -> Result<Artifact> {
	let (artifact,) = args;
	let artifact = artifact.bundle(&client).await?;
	Ok(artifact)
}

fn syscall_checksum(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (checksum::Algorithm, Bytes),
) -> Result<Checksum> {
	let (algorithm, bytes) = args;
	let mut checksum_writer = checksum::Writer::new(algorithm);
	checksum_writer.update(&bytes);
	let checksum = checksum_writer.finalize();
	Ok(checksum)
}

async fn syscall_download(client: Client, args: (Url, Checksum)) -> Result<Blob> {
	todo!()
}

fn syscall_encoding_base64_decode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (String,),
) -> Result<Bytes> {
	let (value,) = args;
	let bytes = base64::engine::general_purpose::STANDARD_NO_PAD
		.decode(value)
		.wrap_err("Failed to decode the bytes.")?;
	Ok(bytes.into())
}

fn syscall_encoding_base64_encode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (Bytes,),
) -> Result<String> {
	let (value,) = args;
	let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(value);
	Ok(encoded)
}

fn syscall_encoding_hex_decode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (Bytes,),
) -> Result<String> {
	let (hex,) = args;
	let bytes = hex::decode(hex).wrap_err("Failed to decode the string as hex.")?;
	let string = String::from_utf8(bytes).wrap_err("Failed to decode the bytes as UTF-8.")?;
	Ok(string)
}

fn syscall_encoding_hex_encode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (String,),
) -> Result<Bytes> {
	let (bytes,) = args;
	let hex = hex::encode(bytes);
	let bytes = hex.into_bytes().into();
	Ok(bytes)
}

fn syscall_encoding_json_decode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (String,),
) -> Result<serde_json::Value> {
	let (json,) = args;
	let value = serde_json::from_str(&json).wrap_err("Failed to decode the string as json.")?;
	Ok(value)
}

fn syscall_encoding_json_encode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (serde_json::Value,),
) -> Result<String> {
	let (value,) = args;
	let json = serde_json::to_string(&value).wrap_err("Failed to encode the value.")?;
	Ok(json)
}

fn syscall_encoding_toml_decode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (String,),
) -> Result<serde_toml::Value> {
	let (toml,) = args;
	let value = serde_toml::from_str(&toml).wrap_err("Failed to decode the string as toml.")?;
	Ok(value)
}

fn syscall_encoding_toml_encode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (serde_toml::Value,),
) -> Result<String> {
	let (value,) = args;
	let toml = serde_toml::to_string(&value).wrap_err("Failed to encode the value.")?;
	Ok(toml)
}

fn syscall_encoding_utf8_decode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (Bytes,),
) -> Result<String> {
	let (bytes,) = args;
	let string = String::from_utf8(bytes.as_slice().to_owned())
		.wrap_err("Failed to decode the bytes as UTF-8.")?;
	Ok(string)
}

fn syscall_encoding_utf8_encode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (String,),
) -> Result<Bytes> {
	let (string,) = args;
	let bytes = string.into_bytes().into();
	Ok(bytes)
}

fn syscall_encoding_yaml_decode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (String,),
) -> Result<serde_yaml::Value> {
	let (yaml,) = args;
	let value = serde_yaml::from_str(&yaml).wrap_err("Failed to decode the string as yaml.")?;
	Ok(value)
}

fn syscall_encoding_yaml_encode(
	_scope: &mut v8::HandleScope,
	_client: Client,
	args: (serde_yaml::Value,),
) -> Result<String> {
	let (value,) = args;
	let yaml = serde_yaml::to_string(&value).wrap_err("Failed to encode the value.")?;
	Ok(yaml)
}

async fn syscall_load(client: Client, args: (object::Id,)) -> Result<object::Object> {
	todo!()
}

fn syscall_log(_scope: &mut v8::HandleScope, _client: Client, args: (String,)) -> Result<()> {
	let (string,) = args;
	println!("{string}");
	Ok(())
}

async fn syscall_read(client: Client, args: (Blob,)) -> Result<Bytes> {
	let (blob,) = args;
	let bytes = blob.bytes(&client).await?;
	Ok(bytes.into())
}

async fn syscall_run(client: Client, args: (Task,)) -> Result<Value> {
	todo!()
}

async fn syscall_store(client: Client, args: (object::Object,)) -> Result<object::Id> {
	todo!()
}

async fn syscall_unpack(client: Client, args: (Blob, ArchiveFormat)) -> Result<Artifact> {
	todo!()
}

fn syscall_sync<'s, A, T, F>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: FromV8,
	T: ToV8,
	F: FnOnce(&mut v8::HandleScope<'s>, Client, A) -> Result<T>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the state.
	let state = context.get_slot::<State>(scope).unwrap().clone();

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = from_v8(scope, args.into()).wrap_err("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(scope, state.client.clone(), args)?;

	// Move the value to v8.
	let value = value
		.to_v8(scope)
		.wrap_err("Failed to serialize the value.")?;

	Ok(value)
}

fn syscall_async<'s, A, T, F, Fut>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: FromV8,
	T: ToV8,
	F: FnOnce(Client, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Get the context.
	let context = scope.get_current_context();

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
		let result = syscall_async_inner(context.clone(), state.client.clone(), args, f).await;
		FutureOutput {
			context,
			promise_resolver,
			result,
		}
	};
	let future = Box::pin(future);

	// Add the future to the context's future set.
	state.futures.borrow_mut().push(future);

	Ok(value.into())
}

async fn syscall_async_inner<'s, A, T, F, Fut>(
	context: v8::Global<v8::Context>,
	client: Client,
	args: v8::Global<v8::Array>,
	f: F,
) -> Result<v8::Global<v8::Value>>
where
	A: FromV8,
	T: ToV8,
	F: FnOnce(Client, A) -> Fut,
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
		from_v8(&mut context_scope, args.into()).wrap_err("Failed to deserialize the args.")?
	};

	// Call the function.
	let value = f(client, args).await?;

	// Serialize the value.
	let value = {
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, &context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
		let value = value
			.to_v8(&mut context_scope)
			.wrap_err("Failed to serialize the value.")?;
		v8::Global::new(&mut context_scope, value)
	};

	Ok(value)
}

#[derive(
	Clone,
	Copy,
	Debug,
	serde::Deserialize,
	serde::Serialize,
	tangram_serialize::Deserialize,
	tangram_serialize::Serialize,
)]
#[serde(into = "String", try_from = "String")]
#[tangram_serialize(into = "String", try_from = "String")]
pub enum ArchiveFormat {
	Tar,
	TarBz2,
	TarGz,
	TarXz,
	TarZstd,
	Zip,
}

#[derive(Clone, Copy, Debug)]
pub enum CompressionFormat {
	Bz2,
	Gz,
	Xz,
	Zstd,
}

impl std::fmt::Display for ArchiveFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Tar => {
				write!(f, ".tar")?;
			},
			Self::TarBz2 => {
				write!(f, ".tar.bz2")?;
			},
			Self::TarGz => {
				write!(f, ".tar.gz")?;
			},
			Self::TarXz => {
				write!(f, ".tar.xz")?;
			},
			Self::TarZstd => {
				write!(f, ".tar.zstd")?;
			},
			Self::Zip => {
				write!(f, ".zip")?;
			},
		}
		Ok(())
	}
}

impl std::str::FromStr for ArchiveFormat {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			".tar" => Ok(Self::Tar),
			".tar.bz2" => Ok(Self::TarBz2),
			".tar.gz" => Ok(Self::TarGz),
			".tar.xz" => Ok(Self::TarXz),
			".tar.zstd" => Ok(Self::TarZstd),
			".zip" => Ok(Self::Zip),
			_ => return_error!("Invalid format."),
		}
	}
}

impl std::fmt::Display for CompressionFormat {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let string = match self {
			Self::Bz2 => ".bz2",
			Self::Gz => ".gz",
			Self::Xz => ".xz",
			Self::Zstd => ".zstd",
		};
		write!(f, "{string}")?;
		Ok(())
	}
}

impl std::str::FromStr for CompressionFormat {
	type Err = Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			".bz2" => Ok(Self::Bz2),
			".gz" => Ok(Self::Gz),
			".xz" => Ok(Self::Xz),
			".zstd" => Ok(Self::Zstd),
			_ => return_error!("Invalid compression format."),
		}
	}
}

impl From<ArchiveFormat> for String {
	fn from(value: ArchiveFormat) -> Self {
		value.to_string()
	}
}

impl TryFrom<String> for ArchiveFormat {
	type Error = Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		value.parse()
	}
}

impl ToV8 for ArchiveFormat {
	fn to_v8<'a>(&self, scope: &mut v8::HandleScope<'a>) -> Result<v8::Local<'a, v8::Value>> {
		self.to_string().to_v8(scope)
	}
}

impl FromV8 for ArchiveFormat {
	fn from_v8<'a>(
		scope: &mut v8::HandleScope<'a>,
		value: v8::Local<'a, v8::Value>,
	) -> Result<Self> {
		String::from_v8(scope, value)?.parse()
	}
}
