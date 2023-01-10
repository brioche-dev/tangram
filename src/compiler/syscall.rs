use super::ContextState;
use crate::compiler::{self, File, OpenedFile};
use anyhow::{bail, Context, Result};
use itertools::Itertools;
use std::rc::Rc;

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
		"print" => syscall_sync(scope, args, syscall_print),
		"opened_files" => syscall_sync(scope, args, syscall_opened_files),
		"version" => syscall_sync(scope, args, syscall_version),
		"resolve" => syscall_sync(scope, args, syscall_resolve),
		"load" => syscall_sync(scope, args, syscall_load),
		_ => bail!(r#"Unknown syscall "{name}"."#),
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

#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn syscall_print(
	_scope: &mut v8::HandleScope,
	_state: Rc<ContextState>,
	args: (String,),
) -> Result<()> {
	let (string,) = args;
	eprintln!("{string}");
	Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadOutput {
	text: String,
	version: i32,
}

fn syscall_opened_files(
	_scope: &mut v8::HandleScope,
	state: Rc<ContextState>,
	_args: (),
) -> Result<Vec<compiler::Url>> {
	let main_runtime_handle = state.main_runtime_handle.clone();
	main_runtime_handle.block_on(async move {
		let files = state.compiler.state.files.read().await;
		let urls = files
			.values()
			.filter_map(|file| match file {
				File::Opened(
					opened_file @ OpenedFile {
						url: compiler::Url::Path { .. },
						..
					},
				) => Some(opened_file.url.clone()),
				_ => None,
			})
			.collect();
		Ok(urls)
	})
}

fn syscall_version(
	_scope: &mut v8::HandleScope,
	state: Rc<ContextState>,
	args: (compiler::Url,),
) -> Result<String> {
	let (url,) = args;
	let main_runtime_handle = state.main_runtime_handle.clone();
	main_runtime_handle.block_on(async move {
		let version = state.compiler.get_version(&url).await?;
		Ok(version.to_string())
	})
}

fn syscall_resolve(
	_scope: &mut v8::HandleScope,
	state: Rc<ContextState>,
	args: (String, Option<compiler::Url>),
) -> Result<compiler::Url> {
	let (specifier, referrer) = args;
	let main_runtime_handle = state.main_runtime_handle.clone();
	main_runtime_handle.block_on(async move {
		let url = state
			.compiler
			.resolve(&specifier, referrer.as_ref())
			.await
			.with_context(|| {
				format!(
					r#"Failed to resolve specifier "{specifier}" relative to referrer "{referrer:?}"."#
				)
			})?;
		Ok(url)
	})
}

fn syscall_load(
	_scope: &mut v8::HandleScope,
	state: Rc<ContextState>,
	args: (compiler::Url,),
) -> Result<LoadOutput> {
	let (url,) = args;
	let main_runtime_handle = state.main_runtime_handle.clone();
	main_runtime_handle.block_on(async move {
		let text = state
			.compiler
			.load(&url)
			.await
			.with_context(|| format!(r#"Failed to load from URL "{url}"."#))?;
		let version = state
			.compiler
			.get_version(&url)
			.await
			.with_context(|| format!(r#"Failed to get the version for URL "{url}"."#))?;
		Ok(LoadOutput { text, version })
	})
}
