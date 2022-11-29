use std::path::PathBuf;

fn main() {
	let runtime_thread = std::thread::spawn(build_runtime_snapshot);
	let compiler_runtime_thread = std::thread::spawn(build_compiler_runtime_snapshot);
	runtime_thread.join().unwrap();
	compiler_runtime_thread.join().unwrap();
}

/// Build the v8 snapshot for the runtime.
fn build_runtime_snapshot() {
	let tangram_extension = deno_core::Extension::builder()
		.js(deno_core::include_js_files!(
			prefix "deno:ext/tangram_js_runtime",
			"js/runtime/global.js",
		))
		.build();
	let extensions = vec![tangram_extension];
	let runtime_opts = deno_core::RuntimeOptions {
		will_snapshot: true,
		module_loader: None,
		extensions,
		..Default::default()
	};
	let mut js_runtime = deno_core::JsRuntime::new(runtime_opts);
	let snapshot = js_runtime.snapshot();
	let snapshot_bytes: &[u8] = &snapshot;
	let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
	let snapshot_path = out_dir.join("js_runtime_snapshot");
	std::fs::write(&snapshot_path, snapshot_bytes).unwrap();
}

/// Build the v8 snapshot for the compiler runtime.
fn build_compiler_runtime_snapshot() {
	let extensions = vec![deno_core::Extension::builder()
		.js(deno_core::include_js_files!(
			prefix "deno:ext/tangram_js_compiler_runtime",
			"js/compiler/typescript/typescript.js",
			"js/compiler/runtime/main.js",
		))
		.build()];
	let runtime_opts = deno_core::RuntimeOptions {
		will_snapshot: true,
		module_loader: None,
		extensions,
		..Default::default()
	};
	let mut js_runtime = deno_core::JsRuntime::new(runtime_opts);
	let snapshot = js_runtime.snapshot();
	let snapshot_bytes: &[u8] = &snapshot;
	let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
	let snapshot_path = out_dir.join("js_compiler_runtime_snapshot");
	std::fs::write(&snapshot_path, snapshot_bytes).unwrap();
}
