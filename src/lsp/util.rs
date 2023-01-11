use crate::compiler::ModuleIdentifier;
use anyhow::Result;
use std::path::Path;
use url::Url;

pub async fn from_uri(url: Url) -> Result<ModuleIdentifier> {
	match url.scheme() {
		"file" => ModuleIdentifier::for_path(Path::new(url.path())).await,
		_ => url.try_into(),
	}
}

pub fn to_uri(module_identifier: ModuleIdentifier) -> Url {
	match module_identifier {
		ModuleIdentifier::Path {
			package_path,
			module_path,
		} => {
			let path = package_path.join(module_path);
			format!("file://{}", path.display()).parse().unwrap()
		},

		_ => module_identifier.into(),
	}
}
