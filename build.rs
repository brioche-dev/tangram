static V8_INIT: std::sync::Once = std::sync::Once::new();

fn main() {
	// Get the out dir path.
	let out_dir_path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

	// Initialize V8.
	V8_INIT.call_once(|| {
		let platform = v8::new_default_platform(0, false).make_shared();
		v8::V8::initialize_platform(platform);
		v8::V8::initialize();
	});

	// Create the language service snapshot.
	println!("cargo-rerun-if-changed=assets/language_service.js");
	let path = out_dir_path.join("language_service.heapsnapshot");
	let snapshot = create_snapshot(include_str!("assets/language_service.js"));
	std::fs::write(path, snapshot).unwrap();

	// Create the runtime global snapshot.
	println!("cargo-rerun-if-changed=assets/global.js");
	let path = out_dir_path.join("global.heapsnapshot");
	let snapshot = create_snapshot(include_str!("assets/global.js"));
	std::fs::write(path, snapshot).unwrap();
}

fn create_snapshot(code: &str) -> v8::StartupData {
	// Create the isolate.
	let mut isolate = v8::Isolate::snapshot_creator(None);

	// Create the context.
	let mut handle_scope = v8::HandleScope::new(&mut isolate);
	let context = v8::Context::new(&mut handle_scope);
	handle_scope.set_default_context(context);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Compile and run the code.
	let code = v8::String::new(&mut context_scope, code).unwrap();
	let script = v8::Script::compile(&mut context_scope, code, None).unwrap();
	script.run(&mut context_scope).unwrap();

	// Drop the scopes.
	drop(context_scope);
	drop(handle_scope);

	// Take the snapshot.
	isolate.create_blob(v8::FunctionCodeHandling::Keep).unwrap()
}
