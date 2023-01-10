use std::{cell::RefCell, rc::Rc};

thread_local! {
	pub static THREAD_LOCAL_ISOLATE: Rc<RefCell<v8::OwnedIsolate>> = {
		// Create the isolate.
		let params = v8::CreateParams::default();
		let isolate = Rc::new(RefCell::new(v8::Isolate::new(params)));

		// Configure the isolate.
		isolate.borrow_mut().set_capture_stack_trace_for_uncaught_exceptions(true, 10);

		isolate
	};
}
