use crate::{
	document::Document,
	error::Result,
	instance::Instance,
	module::{self, Module},
	util::fs,
};
use url::Url;

impl Module {
	pub async fn from_lsp(tg: &Instance, url: Url) -> Result<module::Module> {
		match url.scheme() {
			"file" => {
				let document = Document::for_path(tg, fs::Path::new(url.path())).await?;
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
