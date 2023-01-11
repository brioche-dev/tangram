use super::{isolate::THREAD_LOCAL_ISOLATE, syscall::syscall};
use crate::{
	compiler::{Compiler, ModuleIdentifier},
	Cli,
};
use anyhow::{anyhow, Result};
use futures::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use sourcemap::SourceMap;
use std::{cell::RefCell, future::poll_fn, num::NonZeroI32, rc::Rc, task::Poll};

pub struct ContextState {
	pub cli: Cli,
	pub compiler: Compiler,
	pub main_runtime_handle: tokio::runtime::Handle,
	pub modules: Rc<RefCell<Vec<Module>>>,
	pub futures: Rc<RefCell<FuturesUnordered<LocalBoxFuture<'static, FutureOutput>>>>,
}

#[derive(Debug)]
pub struct Module {
	pub identity_hash: NonZeroI32,
	pub module: v8::Global<v8::Module>,
	pub module_identifier: ModuleIdentifier,
	pub source: String,
	pub _transpiled: Option<String>,
	pub source_map: Option<SourceMap>,
}

pub struct FutureOutput {
	pub context: v8::Global<v8::Context>,
	pub promise_resolver: v8::Global<v8::PromiseResolver>,
	pub result: Result<v8::Global<v8::Value>>,
}

pub fn create_context(
	cli: Cli,
	main_runtime_handle: tokio::runtime::Handle,
) -> (v8::Global<v8::Context>, Rc<ContextState>) {
	// Create the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Context::new(&mut handle_scope);

	// Create the context state.
	let context_state = Rc::new(ContextState {
		cli: cli.clone(),
		compiler: Compiler::new(cli),
		main_runtime_handle,
		modules: Rc::new(RefCell::new(Vec::new())),
		futures: Rc::new(RefCell::new(FuturesUnordered::new())),
	});

	// Set the context state on the context.
	context.set_slot(&mut handle_scope, Rc::clone(&context_state));

	// Create a context scope.
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Create the global syscall function.
	let syscall_string = v8::String::new(&mut context_scope, "syscall").unwrap();
	let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
	context
		.global(&mut context_scope)
		.set(&mut context_scope, syscall_string.into(), syscall.into())
		.unwrap();

	// // Create the tg global.
	// let module = load_module(
	// 	&mut context_scope,
	// 	&compiler::Url::new_core("/mod.ts".into()),
	// )
	// .context("Failed to load the core module.")?;
	// evaluate_module(&mut context_scope, module)
	// 	.await
	// 	.context("Failed to evaluate the core module.")?;
	// let tg = module.get_module_namespace();
	// let tg_string = v8::String::new(&mut context_scope, "tg").unwrap();
	// context
	// 	.global(&mut context_scope)
	// 	.set(&mut context_scope, tg_string.into(), tg)
	// 	.unwrap();

	drop(context_scope);

	// Make the context global.
	let context = v8::Global::new(&mut handle_scope, context);

	(context, context_state)
}

pub async fn await_value(
	context: v8::Global<v8::Context>,
	context_state: Rc<ContextState>,
	value: v8::Global<v8::Value>,
) -> Result<v8::Global<v8::Value>> {
	let context = context.clone();
	let value = poll_fn(move |cx| {
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);

		// Poll the context's futures and resolve or reject all that are ready.
		loop {
			// Poll the context's futures.
			let output = match context_state.futures.borrow_mut().poll_next_unpin(cx) {
				Poll::Ready(Some(output)) => output,
				Poll::Ready(None) => break,
				Poll::Pending => return Poll::Pending,
			};
			let FutureOutput {
				context,
				promise_resolver,
				result,
			} = output;

			// Retrieve the thread local isolate and enter the context.
			let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
			let mut isolate = isolate.borrow_mut();
			let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
			let context = v8::Local::new(&mut handle_scope, context);
			let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

			// Resolve or reject the promise.
			let promise_resolver = v8::Local::new(&mut context_scope, promise_resolver);
			match result {
				Ok(value) => {
					// Resolve the promise.
					let value = v8::Local::new(&mut context_scope, value);
					promise_resolver.resolve(&mut context_scope, value);
				},
				Err(error) => {
					// Reject the promise.
					let error = v8::String::new(&mut context_scope, &error.to_string()).unwrap();
					let error = v8::Local::new(&mut context_scope, error);
					promise_resolver.reject(&mut context_scope, error.into());
				},
			};
		}

		// Enter the context.
		let mut isolate = isolate.borrow_mut();
		let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
		let context = v8::Local::new(&mut handle_scope, &context);
		let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

		// Handle the value.
		let value = v8::Local::new(&mut context_scope, value.clone());
		match v8::Local::<v8::Promise>::try_from(value) {
			Err(_) => {
				let value = v8::Global::new(&mut context_scope, value);
				Poll::Ready(Ok::<_, anyhow::Error>(value))
			},

			Ok(promise) => match promise.state() {
				v8::PromiseState::Pending => Poll::Pending,

				v8::PromiseState::Fulfilled => {
					let value = promise.result(&mut context_scope);
					let value = v8::Global::new(&mut context_scope, value);
					Poll::Ready(Ok(value))
				},

				v8::PromiseState::Rejected => {
					let exception = promise.result(&mut context_scope);
					let exception =
						super::exception::render(&mut context_scope, &context_state, exception);
					Poll::Ready(Err(anyhow!("{exception}")))
				},
			},
		}
	})
	.await?;
	Ok(value)
}
