use self::types::{Request, Response};
use super::{Compiler, File, OpenedFile};
use crate::compiler;
use anyhow::{bail, Context, Result};
use std::{fmt::Write, sync::Arc};

pub mod types;

pub struct Runtime {
	isolate: v8::OwnedIsolate,
	context: v8::Global<v8::Context>,
}

struct State {
	compiler: Compiler,
	main_runtime_handle: tokio::runtime::Handle,
}

impl Runtime {
	#[must_use]
	pub fn new(compiler: Compiler, main_runtime_handle: tokio::runtime::Handle) -> Runtime {
		// Create the state.
		let state = Arc::new(State {
			compiler,
			main_runtime_handle,
		});

		// Create the isolate.
		let params = v8::CreateParams::default();
		let mut isolate = v8::Isolate::new(params);
		isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 10);
		isolate.set_slot(state);

		// Create the context.
		let mut handle_scope = v8::HandleScope::new(&mut isolate);
		let context = v8::Context::new(&mut handle_scope);
		let context = v8::Global::new(&mut handle_scope, context);
		drop(handle_scope);

		// Enter the context.
		let mut handle_scope = v8::HandleScope::new(&mut isolate);
		let context = v8::Local::new(&mut handle_scope, &context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Run the main script.
		let source = v8::String::new(&mut context_scope, include_str!("./main.js")).unwrap();
		let script = v8::Script::compile(&mut context_scope, source, None).unwrap();
		script.run(&mut context_scope).unwrap();

		// Add the syscall function to the global.
		let syscall_string = v8::String::new(&mut context_scope, "syscall").unwrap();
		let syscall_function = v8::Function::new(&mut context_scope, syscall).unwrap();
		context
			.global(&mut context_scope)
			.set(
				&mut context_scope,
				syscall_string.into(),
				syscall_function.into(),
			)
			.unwrap();

		// Exit the context.
		let context = v8::Global::new(&mut context_scope, context);
		drop(context_scope);
		drop(handle_scope);

		Runtime { isolate, context }
	}

	pub fn handle(&mut self, request: Request) -> Result<Response> {
		// Enter the context.
		let mut handle_scope = v8::HandleScope::new(&mut self.isolate);
		let context = v8::Local::new(&mut handle_scope, &self.context);
		let mut scope = v8::ContextScope::new(&mut handle_scope, context);

		// Create a scope to call the handle function.
		let mut try_catch_scope = v8::TryCatch::new(&mut scope);

		// Get the handler.
		let main_string = v8::String::new(&mut try_catch_scope, "main").unwrap();
		let default_string = v8::String::new(&mut try_catch_scope, "default").unwrap();
		let main: v8::Local<v8::Object> = context
			.global(&mut try_catch_scope)
			.get(&mut try_catch_scope, main_string.into())
			.unwrap()
			.try_into()
			.unwrap();
		let default: v8::Local<v8::Function> = main
			.get(&mut try_catch_scope, default_string.into())
			.unwrap()
			.try_into()
			.unwrap();

		// Call the handler.
		let receiver = v8::undefined(&mut try_catch_scope).into();
		let request = serde_v8::to_v8(&mut try_catch_scope, request)
			.context("Failed to serialize the request.")?;
		let output = default.call(&mut try_catch_scope, receiver, &[request]);

		// Handle a caught exception from js.
		if try_catch_scope.has_caught() {
			let exception = try_catch_scope.exception().unwrap();
			let mut scope = v8::HandleScope::new(&mut try_catch_scope);
			let exception = exception_to_string(&mut scope, exception);
			bail!("{exception}");
		}

		// If no exception was caught then retrieve the response.
		let output = output.unwrap();
		let response = serde_v8::from_v8(&mut try_catch_scope, output)
			.context("Failed to deserialize the response.")?;

		// Exit the context.
		drop(try_catch_scope);
		drop(scope);

		Ok(response)
	}
}

#[allow(clippy::needless_pass_by_value)]
fn syscall(
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
	// Retrieve the state.
	let state = scope.get_slot::<Arc<State>>().unwrap();
	let state = Arc::clone(state);

	// Get the syscall name.
	let name: String = serde_v8::from_v8(scope, args.get(0)).unwrap();

	// Invoke the syscall.
	match name.as_str() {
		"print" => {
			let string = serde_v8::from_v8(scope, args.get(1))?;
			#[allow(clippy::let_unit_value)]
			let value = syscall_print(state, string)?;
			let value = serde_v8::to_v8(scope, value)?;
			Ok(value)
		},

		"opened_files" => {
			let value = syscall_opened_files(state)?;
			let value = serde_v8::to_v8(scope, value)?;
			Ok(value)
		},

		"version" => {
			let url = serde_v8::from_v8(scope, args.get(1))?;
			let value = syscall_version(state, url)?;
			let value = serde_v8::to_v8(scope, value)?;
			Ok(value)
		},

		"resolve" => {
			let specifier = serde_v8::from_v8(scope, args.get(1))?;
			let referrer = serde_v8::from_v8(scope, args.get(2))?;
			let value = syscall_resolve(state, specifier, referrer)?;
			let value = serde_v8::to_v8(scope, value)?;
			Ok(value)
		},

		"load" => {
			let url = serde_v8::from_v8(scope, args.get(1)).unwrap();
			let value = syscall_load(state, url)?;
			let value = serde_v8::to_v8(scope, value)?;
			Ok(value)
		},

		_ => unreachable!(),
	}
}

#[allow(clippy::unnecessary_wraps, clippy::needless_pass_by_value)]
fn syscall_print(_state: Arc<State>, string: String) -> Result<()> {
	eprintln!("{string}");
	Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadOutput {
	text: String,
	version: i32,
}

fn syscall_opened_files(state: Arc<State>) -> Result<Vec<compiler::Url>> {
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

fn syscall_version(state: Arc<State>, url: compiler::Url) -> Result<String> {
	let main_runtime_handle = state.main_runtime_handle.clone();
	main_runtime_handle.block_on(async move {
		let version = state.compiler.get_version(&url).await?;
		Ok(version.to_string())
	})
}

fn syscall_resolve(
	state: Arc<State>,
	specifier: String,
	referrer: Option<compiler::Url>,
) -> Result<compiler::Url> {
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

fn syscall_load(state: Arc<State>, url: compiler::Url) -> Result<LoadOutput> {
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

/// Render an exception to a string. The string will include the exception's message and a stack trace.
fn exception_to_string(scope: &mut v8::HandleScope, exception: v8::Local<v8::Value>) -> String {
	let mut string = String::new();

	// Write the exception message.
	let message = exception
		.to_string(scope)
		.unwrap()
		.to_rust_string_lossy(scope);
	writeln!(&mut string, "{message}").unwrap();

	// Write the stack trace if one is available.
	if let Some(stack_trace) = v8::Exception::get_stack_trace(scope, exception) {
		// Write the stack trace one frame at a time.
		for i in 0..stack_trace.get_frame_count() {
			// Retrieve the line, and column.
			let stack_trace_frame = stack_trace.get_frame(scope, i).unwrap();
			let line = stack_trace_frame.get_line_number();
			let column = stack_trace_frame.get_column();

			// Write the URL, line, and column.
			write!(string, "{line}:{column}").unwrap();

			// Add a newline if this is not the last frame.
			if i < stack_trace.get_frame_count() - 1 {
				writeln!(&mut string).unwrap();
			}
		}
	}

	string
}
