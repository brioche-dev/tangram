use super::state::State;
use std::{cell::RefCell, rc::Rc};

pub const SNAPSHOT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/global.heapsnapshot"));

pub const SOURCE_MAP: &[u8] =
	include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/global.js.map"));

thread_local! {
	pub static THREAD_LOCAL_ISOLATE: Rc<RefCell<v8::OwnedIsolate>> = {
		// Create the isolate params.
		let params = v8::CreateParams::default().snapshot_blob(SNAPSHOT);

		// Create the isolate.
		let mut isolate = v8::Isolate::new(params);

		// Set the host import meta object callback.
		isolate.set_host_initialize_import_meta_object_callback(host_initialize_import_meta_object_callback);

		Rc::new(RefCell::new(isolate))
	};
}

pub extern "C" fn host_initialize_import_meta_object_callback(
	context: v8::Local<v8::Context>,
	module: v8::Local<v8::Module>,
	meta: v8::Local<v8::Object>,
) {
	// Create the scope.
	let mut scope = unsafe { v8::CallbackScope::new(context) };

	// Get the state.
	let state = context.get_slot::<Rc<State>>(&mut scope).unwrap().clone();

	// Get the module.
	let identity_hash = module.get_identity_hash();
	let module = state
		.modules
		.borrow()
		.iter()
		.find(|module| module.v8_identity_hash == identity_hash)
		.unwrap()
		.module
		.clone();
	let module = serde_v8::to_v8(&mut scope, module).unwrap();

	// Set import.meta.module.
	let module_string = v8::String::new(&mut scope, "module").unwrap();
	meta.set(&mut scope, module_string.into(), module).unwrap();
}
