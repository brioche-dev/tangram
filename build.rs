fn main() {
	println!("cargo:rerun-if-changed=build.rs");

	#[cfg(feature = "server")]
	{
		// Initialize V8.
		static V8_INIT: std::sync::Once = std::sync::Once::new();
		V8_INIT.call_once(|| {
			let platform = v8::new_default_platform(0, false).make_shared();
			v8::V8::initialize_platform(platform);
			v8::V8::initialize();
		});
	}

	#[cfg(feature = "server")]
	{
		// Create the lsp snapshot.
		let out_dir_path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
		println!("cargo:rerun-if-changed=assets/lsp.js");
		let path = out_dir_path.join("lsp.heapsnapshot");
		let snapshot = create_snapshot("assets/lsp.js");
		std::fs::write(path, snapshot).unwrap();
	}

	#[cfg(feature = "server")]
	{
		// Create the runtime snapshot.
		let out_dir_path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
		println!("cargo:rerun-if-changed=assets/runtime.js");
		let path = out_dir_path.join("runtime.heapsnapshot");
		let snapshot = create_snapshot("assets/runtime.js");
		std::fs::write(path, snapshot).unwrap();
	}
}

#[cfg(feature = "server")]
fn create_snapshot(path: impl AsRef<std::path::Path>) -> v8::StartupData {
	// Create the isolate.
	let mut isolate = v8::Isolate::snapshot_creator(None);

	// Create the context.
	let mut handle_scope = v8::HandleScope::new(&mut isolate);
	let context = v8::Context::new(&mut handle_scope);
	handle_scope.set_default_context(context);
	let mut context_scope = v8::ContextScope::new(&mut handle_scope, context);

	// Compile and run the code.
	let code = std::fs::read_to_string(path).unwrap();
	let code = v8::String::new(&mut context_scope, &code).unwrap();
	let resource_name =
		v8::String::new_external_onebyte_static(&mut context_scope, "[runtime]".as_bytes())
			.unwrap();
	let resource_line_offset = 0;
	let resource_column_offset = 0;
	let resource_is_shared_cross_origin = false;
	let script_id = 0;
	let source_map_url = v8::undefined(&mut context_scope).into();
	let resource_is_opaque = true;
	let is_wasm = false;
	let is_module = false;
	let origin = v8::ScriptOrigin::new(
		&mut context_scope,
		resource_name.into(),
		resource_line_offset,
		resource_column_offset,
		resource_is_shared_cross_origin,
		script_id,
		source_map_url,
		resource_is_opaque,
		is_wasm,
		is_module,
	);
	let script = v8::Script::compile(&mut context_scope, code, Some(&origin)).unwrap();
	script.run(&mut context_scope).unwrap();

	// Drop the scopes.
	drop(context_scope);
	drop(handle_scope);

	// Create the snapshot.
	isolate.create_blob(v8::FunctionCodeHandling::Keep).unwrap()
}
