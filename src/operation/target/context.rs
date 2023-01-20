use super::{isolate::THREAD_LOCAL_ISOLATE, syscall::syscall};
use crate::{compiler::ModuleIdentifier, Cli};
use anyhow::{anyhow, Result};
use futures::{future::LocalBoxFuture, stream::FuturesUnordered, StreamExt};
use sourcemap::SourceMap;
use std::{cell::RefCell, future::poll_fn, num::NonZeroI32, rc::Rc, task::Poll};

pub struct State {
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

pub fn create_context(cli: Cli) -> (v8::Global<v8::Context>, Rc<State>) {
	// Create the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Context::new(&mut handle_scope);

	// Set the cli on the context.
	context.set_slot(&mut handle_scope, cli);

	// Create the state.
	let state = Rc::new(State {
		modules: Rc::new(RefCell::new(Vec::new())),
		futures: Rc::new(RefCell::new(FuturesUnordered::new())),
	});

	// Set the state on the context.
	context.set_slot(&mut handle_scope, Rc::clone(&state));

	// Create a context scope.
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Create the syscall function.
	let syscall_string = v8::String::new(&mut context_scope, "syscall").unwrap();
	let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
	let global = context.global(&mut context_scope);
	global
		.set(&mut context_scope, syscall_string.into(), syscall.into())
		.unwrap();

	// Drop the context scope.
	drop(context_scope);

	// Make the context global.
	let context = v8::Global::new(&mut handle_scope, context);

	(context, state)
}

pub async fn await_value(
	context: v8::Global<v8::Context>,
	state: Rc<State>,
	value: v8::Global<v8::Value>,
) -> Result<v8::Global<v8::Value>> {
	let context = context.clone();
	let value = poll_fn(move |cx| {
		let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);

		// Poll the context's futures and resolve or reject all that are ready.
		loop {
			// Poll the context's futures.
			let output = match state.futures.borrow_mut().poll_next_unpin(cx) {
				Poll::Ready(Some(output)) => output,
				Poll::Ready(None) => break,
				Poll::Pending => return Poll::Pending,
			};
			let FutureOutput {
				context,
				promise_resolver,
				result,
			} = output;

			// Get the thread local isolate and enter the context.
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
					let exception = super::exception::render(&mut context_scope, &state, exception);
					Poll::Ready(Err(anyhow!("{exception}")))
				},
			},
		}
	})
	.await?;
	Ok(value)
}
