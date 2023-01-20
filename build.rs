use std::path::PathBuf;

static V8_INIT: std::sync::Once = std::sync::Once::new();

fn main() {
	// Initialize v8.
	V8_INIT.call_once(|| {
		let platform = v8::new_default_platform(0, false).make_shared();
		v8::V8::initialize_platform(platform);
		v8::V8::initialize();
	});

	// Create the compiler snapshot.
	println!("cargo-rerun-if-changed=src/compiler/mod.js");
	snapshot(include_str!("src/compiler/mod.js"), "compiler.heapsnapshot");

	// Create the runtime snapshot.
	println!("cargo-rerun-if-changed=src/global/mod.js");
	snapshot(include_str!("src/global/mod.js"), "runtime.heapsnapshot");
}

fn snapshot(code: &str, name: &str) {
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

	// Take a snapshot.
	let snapshot = isolate.create_blob(v8::FunctionCodeHandling::Keep).unwrap();

	// Write the snapshot.
	let out_dir_path = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
	let snapshot_path = out_dir_path.join(name);
	std::fs::write(snapshot_path, snapshot).unwrap();
}
