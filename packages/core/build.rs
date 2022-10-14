use std::path::PathBuf;

fn main() {
	let tangram_extension = deno_core::Extension::builder()
		.js(deno_core::include_js_files!(
			prefix "deno:ext/tangram",
			"builder/evaluate/js/global.js",
		))
		.build();
	let extensions = vec![
		deno_webidl::init(),
		deno_url::init(),
		deno_web::init::<Permissions>(deno_web::BlobStore::default(), None),
		tangram_extension,
	];
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
	let snapshot_path = out_dir.join("snapshot");
	std::fs::write(&snapshot_path, snapshot_bytes).unwrap();
}

struct Permissions;

impl deno_web::TimersPermission for Permissions {
	fn allow_hrtime(&mut self) -> bool {
		unreachable!()
	}

	fn check_unstable(&self, _state: &deno_core::OpState, _api_name: &'static str) {
		unreachable!()
	}
}
