use crate::js;

pub fn to_uri(url: js::Url) -> url::Url {
	match url {
		js::Url::PathModule(js::compiler::url::PathModule {
			package_path,
			module_path,
		}) => {
			let path = package_path.join(module_path);
			format!("file://{}", path.display()).parse().unwrap()
		},

		_ => url.into(),
	}
}
