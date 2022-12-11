use super::{ContextState, FutureOutput, IsolateState, THREAD_LOCAL_ISOLATE};
use crate::{compiler, expression::Expression, hash::Hash};
use anyhow::{bail, Context, Result};
use itertools::Itertools;
use std::{cell::RefCell, future::Future, rc::Rc};

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

#[allow(clippy::too_many_lines)]
fn syscall_inner<'s>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
) -> Result<v8::Local<'s, v8::Value>> {
	// Get the syscall name.
	let name: String =
		serde_v8::from_v8(scope, args.get(0)).context("Failed to deserialize the syscall name.")?;

	// Invoke the syscall.
	match name.as_str() {
		"get_hash" => syscall_sync(scope, args, syscall_get_hash),
		"get_name" => syscall_sync(scope, args, syscall_get_name),
		"get_args" => syscall_sync(scope, args, syscall_get_args),
		"return" => syscall_sync(scope, args, syscall_return),
		"print" => syscall_sync(scope, args, syscall_print),
		"serialize" => syscall_sync(scope, args, syscall_serialize),
		"deserialize" => syscall_sync(scope, args, syscall_deserialize),
		"add_blob" => syscall_async(scope, args, syscall_add_blob),
		"get_blob" => syscall_async(scope, args, syscall_get_blob),
		"add_expression" => syscall_async(scope, args, syscall_add_expression),
		"get_expression" => syscall_async(scope, args, syscall_get_expression),
		"evaluate" => syscall_async(scope, args, syscall_evaluate),
		_ => {
			bail!(r#"Unknown syscall "{name}"."#);
		},
	}
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_get_hash(_state: Rc<RefCell<ContextState>>, args: (compiler::Url,)) -> Result<Hash> {
	let (url,) = args;
	let package_hash = match url {
		compiler::Url::HashModule(compiler::url::HashModule { package_hash, .. }) => package_hash,
		_ => bail!("Invalid URL."),
	};
	Ok(package_hash)
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_get_name(state: Rc<RefCell<ContextState>>, _args: ()) -> Result<String> {
	Ok(state.borrow().name.as_ref().cloned().unwrap())
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_get_args(state: Rc<RefCell<ContextState>>, _args: ()) -> Result<Hash> {
	Ok(state.borrow().args.as_ref().copied().unwrap())
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_return(state: Rc<RefCell<ContextState>>, args: (Hash,)) -> Result<()> {
	let (value,) = args;
	state.borrow_mut().output.replace(value);
	Ok(())
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_print(_state: Rc<RefCell<ContextState>>, args: (String,)) -> Result<()> {
	let (string,) = args;
	println!("{string}");
	Ok(())
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
enum SerializationFormat {
	#[serde(rename = "toml")]
	Toml,
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_serialize(
	_state: Rc<RefCell<ContextState>>,
	args: (SerializationFormat, serde_json::Value),
) -> Result<String> {
	let (format, value) = args;
	match format {
		SerializationFormat::Toml => {
			let value = toml::to_string(&value)?;
			Ok(value)
		},
	}
}

#[allow(clippy::needless_pass_by_value, clippy::unnecessary_wraps)]
fn syscall_deserialize(
	_state: Rc<RefCell<ContextState>>,
	args: (SerializationFormat, String),
) -> Result<serde_json::Value> {
	let (format, string) = args;
	match format {
		SerializationFormat::Toml => {
			let value = toml::from_str(&string)?;
			Ok(value)
		},
	}
}

async fn syscall_add_blob(
	state: Rc<RefCell<ContextState>>,
	args: (serde_v8::ZeroCopyBuf,),
) -> Result<Hash> {
	let (blob,) = args;
	let cli = state.borrow().cli.clone();
	let cli = cli.lock_shared().await?;
	let hash = cli.add_blob(blob.as_ref()).await?;
	Ok(hash)
}

async fn syscall_get_blob(
	state: Rc<RefCell<ContextState>>,
	args: (Hash,),
) -> Result<serde_v8::ZeroCopyBuf> {
	let (hash,) = args;
	let cli = state.borrow().cli.clone();
	let cli = cli.lock_shared().await?;
	let mut blob = cli.get_blob(hash).await?;
	let mut bytes = Vec::new();
	tokio::io::copy(&mut blob, &mut bytes).await?;
	let output = serde_v8::ZeroCopyBuf::ToV8(Some(bytes.into_boxed_slice()));
	Ok(output)
}

async fn syscall_add_expression(
	state: Rc<RefCell<ContextState>>,
	args: (Expression,),
) -> Result<Hash> {
	let (expression,) = args;
	let cli = state.borrow().cli.clone();
	let cli = cli.lock_shared().await?;
	let hash = cli.add_expression(&expression).await?;
	Ok(hash)
}

async fn syscall_get_expression(
	state: Rc<RefCell<ContextState>>,
	args: (Hash,),
) -> Result<Option<Expression>> {
	let (hash,) = args;
	let cli = state.borrow().cli.clone();
	let cli = cli.lock_shared().await?;
	let expression = cli.try_get_expression_local(hash)?;
	Ok(expression)
}

async fn syscall_evaluate(state: Rc<RefCell<ContextState>>, args: (Hash,)) -> Result<Hash> {
	let (hash,) = args;
	let cli = state.borrow().cli.clone();
	let cli = cli.lock_shared().await?;
	let output = cli.evaluate(hash, hash).await?;
	Ok(output)
}

fn syscall_sync<'s, A, T, F>(
	scope: &mut v8::HandleScope<'s>,
	args: &v8::FunctionCallbackArguments,
	f: F,
) -> Result<v8::Local<'s, v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Rc<RefCell<ContextState>>, A) -> Result<T>,
{
	// Retrieve the context and context state.
	let context = scope.get_current_context();
	let context_state = Rc::clone(
		context
			.get_slot::<Rc<RefCell<ContextState>>>(scope)
			.unwrap(),
	);

	// Collect the args.
	let args = (1..args.length()).map(|i| args.get(i)).collect_vec();
	let args = v8::Array::new_with_elements(scope, args.as_slice());

	// Deserialize the args.
	let args = serde_v8::from_v8(scope, args.into()).context("Failed to deserialize the args.")?;

	// Call the function.
	let value = f(context_state, args)?;

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
	F: FnOnce(Rc<RefCell<ContextState>>, A) -> Fut + 'static,
	Fut: Future<Output = Result<T>>,
{
	// Retrieve the isolate state, context, and context state.
	let isolate_state = Rc::clone(scope.get_slot::<Rc<RefCell<IsolateState>>>().unwrap());
	let context = scope.get_current_context();
	let context_state = Rc::clone(
		context
			.get_slot::<Rc<RefCell<ContextState>>>(scope)
			.unwrap(),
	);

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
		let result = syscall_async_inner(context.clone(), context_state, args, f).await;
		FutureOutput {
			context,
			promise_resolver,
			result,
		}
	});

	// Add the future to the isolate's set of futures.
	isolate_state.borrow_mut().futures.push(future);

	Ok(value.into())
}

async fn syscall_async_inner<A, T, F, Fut>(
	context: v8::Global<v8::Context>,
	context_state: Rc<RefCell<ContextState>>,
	args: v8::Global<v8::Array>,
	f: F,
) -> Result<v8::Global<v8::Value>>
where
	A: serde::de::DeserializeOwned,
	T: serde::Serialize,
	F: FnOnce(Rc<RefCell<ContextState>>, A) -> Fut + 'static,
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
	let value = f(context_state, args).await?;

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
