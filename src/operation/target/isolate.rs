use std::{cell::RefCell, rc::Rc};

const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/runtime.heapsnapshot"));

thread_local! {
	pub static THREAD_LOCAL_ISOLATE: Rc<RefCell<v8::OwnedIsolate>> = {
		// Create the isolate params.
		let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);

		// Create the isolate.
		Rc::new(RefCell::new(v8::Isolate::new(params)))
	};
}
