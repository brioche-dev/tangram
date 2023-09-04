use crate::{
	document::Document,
	error::Result,
	module::{self, Module},
	server::Server,
};
use std::path::Path;
use url::Url;

impl Module {
	pub async fn from_lsp(tg: &Server, url: Url) -> Result<module::Module> {
		match url.scheme() {
			"file" => {
				let document = Document::for_path(tg, Path::new(url.path())).await?;
				let module = Module::Document(document);
				Ok(module)
			},
			_ => url.try_into(),
		}
	}

	#[must_use]
	pub fn to_lsp(&self) -> Url {
		match self {
			Module::Document(document) => {
				let path = document.package_path.join(document.module_path.to_string());
				let path = path.display();
				format!("file://{path}").parse().unwrap()
			},

			_ => self.clone().into(),
		}
	}
}
