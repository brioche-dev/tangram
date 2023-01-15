use super::{ModuleIdentifier, OpenedTrackedFile, TrackedFile};
use crate::Cli;
use anyhow::{bail, Context, Result};
use itertools::Itertools;

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
	F: FnOnce(&Cli, &mut v8::HandleScope<'s>, A) -> Result<T>,
{
	// Get the context.
	let context = scope.get_current_context();

	// Get the cli.
	let cli = context.get_slot::<Cli>(scope).unwrap().clone();

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = serde_v8::from_v8(scope, args.into()).context("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(&cli, scope, args)?;

	// Serialize the value.
	let value = serde_v8::to_v8(scope, &value).context("Failed to serialize the value.")?;

	Ok(value)
}

#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn syscall_print(_cli: &Cli, _scope: &mut v8::HandleScope, args: (String,)) -> Result<()> {
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
	cli: &Cli,
	_scope: &mut v8::HandleScope,
	_args: (),
) -> Result<Vec<ModuleIdentifier>> {
	cli.inner.runtime_handle.clone().block_on(async move {
		let tracked_files = cli.inner.tracked_files.read().await;
		let module_identifiers = tracked_files
			.values()
			.filter_map(|tracked_file| match tracked_file {
				TrackedFile::Opened(
					opened_file @ OpenedTrackedFile {
						module_identifier: ModuleIdentifier::Path { .. },
						..
					},
				) => Some(opened_file.module_identifier.clone()),
				_ => None,
			})
			.collect();
		Ok(module_identifiers)
	})
}

fn syscall_version(
	cli: &Cli,
	_scope: &mut v8::HandleScope,
	args: (ModuleIdentifier,),
) -> Result<String> {
	let (url,) = args;
	cli.inner.runtime_handle.clone().block_on(async move {
		let version = cli.version(&url).await?;
		Ok(version.to_string())
	})
}

fn syscall_resolve(
	cli: &Cli,
	_scope: &mut v8::HandleScope,
	args: (String, Option<ModuleIdentifier>),
) -> Result<ModuleIdentifier> {
	let (specifier, referrer) = args;
	cli.inner.runtime_handle.clone().block_on(async move {
		let module_identifier = cli
			.resolve(&specifier, referrer.as_ref())
			.await
			.with_context(|| {
				format!(
					r#"Failed to resolve specifier "{specifier}" relative to referrer "{referrer:?}"."#
				)
			})?;
		Ok(module_identifier)
	})
}

fn syscall_load(
	cli: &Cli,
	_scope: &mut v8::HandleScope,
	args: (ModuleIdentifier,),
) -> Result<LoadOutput> {
	let (module_identifier,) = args;
	cli.inner.runtime_handle.clone().block_on(async move {
		let text = cli
			.load(&module_identifier)
			.await
			.with_context(|| format!(r#"Failed to load module "{module_identifier}"."#))?;
		let version = cli.version(&module_identifier).await.with_context(|| {
			format!(r#"Failed to get the version for module "{module_identifier}"."#)
		})?;
		Ok(LoadOutput { text, version })
	})
}
