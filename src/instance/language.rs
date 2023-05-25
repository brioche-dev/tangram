
/// State associated with an `Instance` required to provide language support.
pub (crate) struct Language {
	/// A handle to the main tokio runtime.
	pub(crate) runtime: tokio::runtime::Handle,

	/// A local pool for running `!Send` futures.
	pub(crate) local_pool: tokio_util::task::LocalPoolHandle,

	/// A channel sender to send requests to the language service.
	pub(crate) service_request_sender: std::sync::Mutex<Option<crate::language::service::RequestSender>>,
}


static V8_INIT: std::sync::Once = std::sync::Once::new();

fn initialize_v8() {
	// Set the ICU data.
	#[repr(C, align(16))]
	struct IcuData([u8; 10_541_264]);
	static ICU_DATA: IcuData = IcuData(*include_bytes!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/assets/icudtl.dat"
	)));
	v8::icu::set_common_data_72(&ICU_DATA.0).unwrap();

	// Initialize the platform.
	let platform = v8::new_default_platform(0, true);
	v8::V8::initialize_platform(platform.make_shared());

	// Initialize V8.
	v8::V8::initialize();
}

impl Language {
    pub fn new () -> Language {
		// Initialize v8.
		V8_INIT.call_once(initialize_v8);

		// Get the curent tokio runtime handler.
		let runtime = tokio::runtime::Handle::current();

		// Create the local pool handle.
		let threads = std::thread::available_parallelism().unwrap().get();
		let local_pool = tokio_util::task::LocalPoolHandle::new(threads);

		// Create a new sender for the service request.
		let service_request_sender = std::sync::Mutex::new(None);

		Language { runtime, local_pool, service_request_sender }
    }
}

