#![allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]

use super::{
	convert::{from_v8, FromV8, ToV8},
	State,
};
use crate::{
	blob, checksum, object, return_error, util::tokio_util::SyncIoBridge, Artifact, Blob, Bytes,
	Checksum, Result, Target, Value, WrapErr,
};
use base64::Engine as _;
use futures::TryStreamExt;
use itertools::Itertools;
use std::{future::Future, rc::Rc};
use tokio::io::AsyncRead;
use tokio_util::io::StreamReader;
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
			// Throw an exception.
			let exception = error.to_v8(scope).unwrap();
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
		"build" => syscall_async(scope, args, syscall_build),
		"bundle" => syscall_async(scope, args, syscall_bundle),
		"checksum" => syscall_sync(scope, args, syscall_checksum),
		"decompress" => syscall_async(scope, args, syscall_decompress),
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
		"extract" => syscall_async(scope, args, syscall_extract),
		"load" => syscall_async(scope, args, syscall_load),
		"log" => syscall_sync(scope, args, syscall_log),
		"read" => syscall_async(scope, args, syscall_read),
		"store" => syscall_async(scope, args, syscall_store),
		_ => return_error!(r#"Unknown syscall "{name}"."#),
	}
}

async fn syscall_build(state: Rc<State>, args: (Target,)) -> Result<Value> {
	let (target,) = args;
	let build = target.build(&state.client).await?;
	let Some(output) = build.output(&state.client).await? else {
		return_error!("The build failed.");
	};
	Ok(output)
}

async fn syscall_bundle(state: Rc<State>, args: (Artifact,)) -> Result<Artifact> {
	let (artifact,) = args;
	let artifact = artifact.bundle(&state.client).await?;
	Ok(artifact)
}

fn syscall_checksum(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (checksum::Algorithm, Bytes),
) -> Result<Checksum> {
	let (algorithm, bytes) = args;
	let mut checksum_writer = checksum::Writer::new(algorithm);
	checksum_writer.update(&bytes);
	let checksum = checksum_writer.finalize();
	Ok(checksum)
}

async fn syscall_decompress(
	state: Rc<State>,
	args: (Blob, blob::CompressionFormat),
) -> Result<Blob> {
	let (blob, format) = args;
	let reader = blob.reader(&state.client).await?;
	let reader = tokio::io::BufReader::new(reader);
	let reader: Box<dyn AsyncRead + Unpin> = match format {
		blob::CompressionFormat::Bz2 => {
			Box::new(async_compression::tokio::bufread::BzDecoder::new(reader))
		},
		blob::CompressionFormat::Gz => {
			Box::new(async_compression::tokio::bufread::GzipDecoder::new(reader))
		},
		blob::CompressionFormat::Xz => {
			Box::new(async_compression::tokio::bufread::XzDecoder::new(reader))
		},
		blob::CompressionFormat::Zstd => {
			Box::new(async_compression::tokio::bufread::ZstdDecoder::new(reader))
		},
	};
	let blob = Blob::with_reader(&state.client, reader).await?;
	Ok(blob)
}

async fn syscall_download(state: Rc<State>, args: (Url, Checksum)) -> Result<Blob> {
	let (url, checksum) = args;
	let response = reqwest::get(url).await?.error_for_status()?;
	let mut checksum_writer = checksum::Writer::new(checksum.algorithm());
	let stream = response
		.bytes_stream()
		.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
		.inspect_ok(|chunk| checksum_writer.update(chunk));
	let blob = Blob::with_reader(&state.client, StreamReader::new(stream)).await?;
	let actual = checksum_writer.finalize();
	if actual != checksum {
		return_error!(r#"The checksum did not match. Expected "{checksum}" but got "{actual}"."#);
	}
	Ok(blob)
}

fn syscall_encoding_base64_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
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
	_state: Rc<State>,
	args: (Bytes,),
) -> Result<String> {
	let (value,) = args;
	let encoded = base64::engine::general_purpose::STANDARD_NO_PAD.encode(value);
	Ok(encoded)
}

fn syscall_encoding_hex_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<Bytes> {
	let (string,) = args;
	let bytes = hex::decode(string).wrap_err("Failed to decode the string as hex.")?;
	Ok(bytes.into())
}

fn syscall_encoding_hex_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (Bytes,),
) -> Result<String> {
	let (bytes,) = args;
	let hex = hex::encode(bytes);
	Ok(hex)
}

fn syscall_encoding_json_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_json::Value> {
	let (json,) = args;
	let value = serde_json::from_str(&json).wrap_err("Failed to decode the string as json.")?;
	Ok(value)
}

fn syscall_encoding_json_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_json::Value,),
) -> Result<String> {
	let (value,) = args;
	let json = serde_json::to_string(&value).wrap_err("Failed to encode the value.")?;
	Ok(json)
}

fn syscall_encoding_toml_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_toml::Value> {
	let (toml,) = args;
	let value = serde_toml::from_str(&toml).wrap_err("Failed to decode the string as toml.")?;
	Ok(value)
}

fn syscall_encoding_toml_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_toml::Value,),
) -> Result<String> {
	let (value,) = args;
	let toml = serde_toml::to_string(&value).wrap_err("Failed to encode the value.")?;
	Ok(toml)
}

fn syscall_encoding_utf8_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (Bytes,),
) -> Result<String> {
	let (bytes,) = args;
	let string = String::from_utf8(bytes.as_slice().to_owned())
		.wrap_err("Failed to decode the bytes as UTF-8.")?;
	Ok(string)
}

fn syscall_encoding_utf8_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<Bytes> {
	let (string,) = args;
	let bytes = string.into_bytes().into();
	Ok(bytes)
}

fn syscall_encoding_yaml_decode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (String,),
) -> Result<serde_yaml::Value> {
	let (yaml,) = args;
	let value = serde_yaml::from_str(&yaml).wrap_err("Failed to decode the string as yaml.")?;
	Ok(value)
}

fn syscall_encoding_yaml_encode(
	_scope: &mut v8::HandleScope,
	_state: Rc<State>,
	args: (serde_yaml::Value,),
) -> Result<String> {
	let (value,) = args;
	let yaml = serde_yaml::to_string(&value).wrap_err("Failed to encode the value.")?;
	Ok(yaml)
}

async fn syscall_extract(state: Rc<State>, args: (Blob, blob::ArchiveFormat)) -> Result<Artifact> {
	let (blob, format) = args;

	// Create the reader.
	let reader = blob.reader(&state.client).await?;

	// Create a temp.
	let tempdir = tempfile::TempDir::new()?;
	let path = tempdir.path().join("archive");

	// Extract in a blocking task.
	tokio::task::spawn_blocking({
		let reader = SyncIoBridge::new(reader);
		let path = path.clone();
		move || -> Result<_> {
			match format {
				blob::ArchiveFormat::Tar => {
					let mut archive = tar::Archive::new(reader);
					archive.set_preserve_permissions(false);
					archive.set_unpack_xattrs(false);
					archive.unpack(path)?;
				},
				blob::ArchiveFormat::Zip => {
					let mut archive =
						zip::ZipArchive::new(reader).wrap_err("Failed to read the zip archive.")?;
					archive
						.extract(&path)
						.wrap_err("Failed to extract the zip archive.")?;
				},
			}
			Ok(())
		}
	})
	.await
	.unwrap()?;

	// Check in the unpack temp path.
	let artifact = Artifact::check_in(&state.client, &path)
		.await
		.wrap_err("Failed to check in the extracted archive.")?;

	Ok(artifact)
}

async fn syscall_load(state: Rc<State>, args: (object::Id,)) -> Result<object::Object> {
	let (id,) = args;
	object::Handle::with_id(id)
		.object(&state.client)
		.await
		.cloned()
}

fn syscall_log(_scope: &mut v8::HandleScope, state: Rc<State>, args: (String,)) -> Result<()> {
	let (string,) = args;
	eprintln!("{string}");
	state.progress.add_log(string.into_bytes());
	Ok(())
}

async fn syscall_read(state: Rc<State>, args: (Blob,)) -> Result<Bytes> {
	let (blob,) = args;
	let bytes = blob.bytes(&state.client).await?;
	Ok(bytes.into())
}

async fn syscall_store(state: Rc<State>, args: (object::Object,)) -> Result<object::Id> {
	let (object,) = args;
	object::Handle::with_object(object).id(&state.client).await
}

fn syscall_sync<'s, A, T, F>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: FromV8,
	T: ToV8,
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
	let args = from_v8(scope, args.into()).wrap_err("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(scope, state, args)?;

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
	A: FromV8 + 'static,
	T: ToV8 + 'static,
	F: FnOnce(Rc<State>, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the state.
	let state = context.get_slot::<Rc<State>>(scope).unwrap().clone();

	// Create the promise.
	let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
	let promise = promise_resolver.get_promise(scope);

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = from_v8(scope, args.into()).wrap_err("Failed to deserialize the args.")?;

	// Move the promise resolver to the global scope.
	let promise_resolver = v8::Global::new(scope, promise_resolver);

	// Create the future.
	let future = {
		let state = state.clone();
		async move {
			let result = f(state, args)
				.await
				.map(|value| Box::new(value) as Box<dyn ToV8>);
			(result, promise_resolver)
		}
	};
	let future = Box::pin(future);

	// Add the future to the context's futures.
	state.futures.borrow_mut().push(future);

	Ok(promise.into())
}
