use crate::{
	error::{return_error, Error, Result, WrapErr},
	instance::Instance,
	module::{self, Module},
};
use itertools::Itertools;
use std::sync::Weak;

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
			// Throw an exception.
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
	let name: String = serde_v8::from_v8(scope, args.get(0)).unwrap();

	// Invoke the syscall.
	match name.as_str() {
		"documents" => syscall_sync(scope, args, syscall_documents),
		"hex_decode" => syscall_sync(scope, args, syscall_hex_decode),
		"hex_encode" => syscall_sync(scope, args, syscall_hex_encode),
		"json_decode" => syscall_sync(scope, args, syscall_json_decode),
		"json_encode" => syscall_sync(scope, args, syscall_json_encode),
		"log" => syscall_sync(scope, args, syscall_log),
		"module_load" => syscall_sync(scope, args, syscall_module_load),
		"module_resolve" => syscall_sync(scope, args, syscall_module_resolve),
		"module_version" => syscall_sync(scope, args, syscall_module_version),
		"utf8_decode" => syscall_sync(scope, args, syscall_utf8_decode),
		"utf8_encode" => syscall_sync(scope, args, syscall_utf8_encode),
		_ => return_error!(r#"Unknown syscall "{name}"."#),
	}
}

fn syscall_documents(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	_args: (),
) -> Result<Vec<module::Module>> {
	tg.language.runtime.clone().block_on(async move {
		let documents = tg.documents.read().await;
		let modules = documents.keys().cloned().map(Module::Document).collect();
		Ok(modules)
	})
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_hex_decode(
	_tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (String,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (hex,) = args;
	let bytes = hex::decode(hex)
		.map_err(Error::other)
		.wrap_err("Failed to decode the string as hex.")?;
	Ok(bytes.into())
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_hex_encode(
	_tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (serde_v8::ZeroCopyBuf,),
) -> Result<String> {
	let (bytes,) = args;
	let hex = hex::encode(bytes);
	Ok(hex)
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_json_decode(
	_tg: &Instance,
	_scope: &mut v8::HandleScope,
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
	_tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (serde_json::Value,),
) -> Result<String> {
	let (value,) = args;
	let json = serde_json::to_string(&value)
		.map_err(Error::other)
		.wrap_err("Failed to encode the value.")?;
	Ok(json)
}

#[allow(clippy::unnecessary_wraps)]
fn syscall_log(_tg: &Instance, _scope: &mut v8::HandleScope, args: (String,)) -> Result<()> {
	let (string,) = args;
	eprintln!("{string}");
	Ok(())
}

fn syscall_module_load(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (module::Module,),
) -> Result<String> {
	let (module,) = args;
	tg.language.runtime.clone().block_on(async move {
		let text = module
			.load(tg)
			.await
			.wrap_err_with(|| format!(r#"Failed to load module "{module}"."#))?;
		Ok(text)
	})
}

fn syscall_module_resolve(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (module::Module, module::Import),
) -> Result<module::Module> {
	let (module, specifier) = args;
	tg.language.runtime.clone().block_on(async move {
		let module = module.resolve(tg, &specifier).await.wrap_err_with(|| {
			format!(r#"Failed to resolve specifier "{specifier}" relative to module "{module}"."#)
		})?;
		Ok(module)
	})
}

fn syscall_module_version(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (module::Module,),
) -> Result<String> {
	let (module,) = args;
	tg.language.runtime.clone().block_on(async move {
		let version = module.version(tg).await?;
		Ok(version.to_string())
	})
}

#[allow(clippy::needless_pass_by_value)]
fn syscall_utf8_decode(
	_tg: &Instance,
	_scope: &mut v8::HandleScope,
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
	_tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (String,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (string,) = args;
	let bytes = string.into_bytes().into();
	Ok(bytes)
}

fn syscall_sync<'s, A, T, F>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(&Instance, &mut v8::HandleScope<'s>, A) -> Result<T>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the instance.
	let tg = context
		.get_slot::<Weak<Instance>>(scope)
		.unwrap()
		.upgrade()
		.unwrap();

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = serde_v8::from_v8(scope, args.into())
		.map_err(Error::other)
		.wrap_err("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(&tg, scope, args)?;

	// Serialize the value.
	let value = serde_v8::to_v8(scope, &value)
		.map_err(Error::other)
		.wrap_err("Failed to serialize the value.")?;

	Ok(value)
}
