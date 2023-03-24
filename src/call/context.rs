use super::{
	isolate::THREAD_LOCAL_ISOLATE,
	state::{FutureOutput, State},
	syscall::syscall,
};
use crate::{
	error::{Error, Result},
	Instance,
};
use futures::{stream::FuturesUnordered, StreamExt};
use std::{cell::RefCell, future::poll_fn, rc::Rc, sync::Arc, task::Poll};

pub fn new(tg: Arc<Instance>) -> v8::Global<v8::Context> {
	// Create the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Context::new(&mut handle_scope);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Set the instance on the context.
	context.set_slot(&mut context_scope, tg);

	// Create the state.
	let state = Rc::new(State {
		modules: Rc::new(RefCell::new(Vec::new())),
		futures: Rc::new(RefCell::new(FuturesUnordered::new())),
	});

	// Set the state on the context.
	context.set_slot(&mut context_scope, Rc::clone(&state));

	// Create the syscall function.
	let syscall_string = v8::String::new(&mut context_scope, "syscall").unwrap();
	let syscall = v8::Function::new(&mut context_scope, syscall).unwrap();
	let global = context.global(&mut context_scope);
	global
		.set(&mut context_scope, syscall_string.into(), syscall.into())
		.unwrap();

	// Drop the context scope.
	drop(context_scope);

	v8::Global::new(&mut handle_scope, context)
}

pub async fn await_value(
	context: v8::Global<v8::Context>,
	value: v8::Global<v8::Value>,
) -> Result<v8::Global<v8::Value>> {
	poll_fn(move |cx| await_value_inner(context.clone(), value.clone(), cx)).await
}

pub fn await_value_inner(
	context: v8::Global<v8::Context>,
	value: v8::Global<v8::Value>,
	cx: &mut std::task::Context<'_>,
) -> Poll<Result<v8::Global<v8::Value>>> {
	// Get the state.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Local::new(&mut handle_scope, context);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);
	let state = Rc::clone(context.get_slot::<Rc<State>>(&mut context_scope).unwrap());
	drop(context_scope);
	let context = v8::Global::new(&mut handle_scope, context);
	drop(handle_scope);
	drop(isolate);

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
				let exception = error.to_exception(&mut context_scope);
				promise_resolver.reject(&mut context_scope, exception);
			},
		};
	}

	// Enter the context.
	let isolate = THREAD_LOCAL_ISOLATE.with(Rc::clone);
	let mut isolate = isolate.borrow_mut();
	let mut handle_scope = v8::HandleScope::new(isolate.as_mut());
	let context = v8::Local::new(&mut handle_scope, context);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Handle the value.
	let value = v8::Local::new(&mut context_scope, value);
	match v8::Local::<v8::Promise>::try_from(value) {
		Err(_) => {
			let value = v8::Global::new(&mut context_scope, value);
			Poll::Ready(Ok::<_, Error>(value))
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
				let error = Error::from_exception(&mut context_scope, &state, exception);
				Poll::Ready(Err(error))
			},
		},
	}
}
