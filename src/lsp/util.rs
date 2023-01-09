use crate::compiler;
use anyhow::Result;
use std::path::Path;

pub async fn from_uri(url: url::Url) -> Result<compiler::Url> {
	match url.scheme() {
		"file" => compiler::Url::for_path(Path::new(url.path())).await,
		_ => url.try_into(),
	}
}

pub fn to_uri(url: compiler::Url) -> url::Url {
	match url {
		compiler::Url::Path {
			package_path,
			module_path,
		} => {
			let path = package_path.join(module_path);
			format!("file://{}", path.display()).parse().unwrap()
		},

		_ => url.into(),
	}
}
