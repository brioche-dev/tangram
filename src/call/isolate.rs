use std::{cell::RefCell, rc::Rc};

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/global.heapsnapshot"));

thread_local! {
	pub static THREAD_LOCAL_ISOLATE: Rc<RefCell<v8::OwnedIsolate>> = {
		// Create the isolate params.
		let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);

		// Create the isolate.
		let mut isolate = v8::Isolate::new(params);
		isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 10);

		Rc::new(RefCell::new(isolate))
	};
}
