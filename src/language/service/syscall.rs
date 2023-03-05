use crate::{module, Instance};
use anyhow::{bail, Context, Result};
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
	let name: String = serde_v8::from_v8(scope, args.get(0)).unwrap();

	// Invoke the syscall.
	match name.as_str() {
		"documents" => syscall_sync(scope, args, syscall_documents),
		"load" => syscall_sync(scope, args, syscall_load),
		"log" => syscall_sync(scope, args, syscall_log),
		"resolve" => syscall_sync(scope, args, syscall_resolve),
		"version" => syscall_sync(scope, args, syscall_version),
		_ => bail!(r#"Unknown syscall "{name}"."#),
	}
}

fn syscall_load(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (module::Identifier,),
) -> Result<String> {
	let (module_identifier,) = args;
	tg.runtime_handle.clone().block_on(async move {
		let text = tg
			.load_document_or_module(&module_identifier)
			.await
			.with_context(|| format!(r#"Failed to load module "{module_identifier}"."#))?;
		Ok(text)
	})
}

#[allow(clippy::unnecessary_wraps)]
fn syscall_log(_tg: &Instance, _scope: &mut v8::HandleScope, args: (String,)) -> Result<()> {
	let (string,) = args;
	eprintln!("{string}");
	Ok(())
}

fn syscall_documents(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	_args: (),
) -> Result<Vec<module::Identifier>> {
	tg.runtime_handle.clone().block_on(async move {
		let documents = tg.documents.read().await;
		let module_identifiers = documents.keys().cloned().collect();
		Ok(module_identifiers)
	})
}

fn syscall_resolve(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (module::Specifier, module::Identifier),
) -> Result<module::Identifier> {
	let (specifier, referrer) = args;
	tg.runtime_handle.clone().block_on(async move {
		let module_identifier = tg
			.resolve_module(&specifier, &referrer)
			.await
			.with_context(|| {
				format!(
					r#"Failed to resolve specifier "{specifier:?}" relative to referrer "{referrer:?}"."#
				)
			})?;
		Ok(module_identifier)
	})
}

fn syscall_version(
	tg: &Instance,
	_scope: &mut v8::HandleScope,
	args: (module::Identifier,),
) -> Result<String> {
	let (module_identifier,) = args;
	tg.runtime_handle.clone().block_on(async move {
		let version = tg
			.get_document_or_module_version(&module_identifier)
			.await?;
		Ok(version.to_string())
	})
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
	let args = serde_v8::from_v8(scope, args.into()).context("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(&tg, scope, args)?;

	// Serialize the value.
	let value = serde_v8::to_v8(scope, &value).context("Failed to serialize the value.")?;

	Ok(value)
}
