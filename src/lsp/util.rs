use crate::{error::Result, module, util::fs};
use url::Url;

impl module::Identifier {
	pub async fn from_lsp_uri(url: Url) -> Result<module::Identifier> {
		match url.scheme() {
			"file" => module::Identifier::for_path(fs::Path::new(url.path())).await,
			_ => url.try_into(),
		}
	}

	#[must_use]
	pub fn to_lsp_uri(&self) -> Url {
		match &self.source {
			module::identifier::Source::Path(package_path) => {
				let path = package_path.join(self.path.to_string());
				let path = path.display();
				format!("file://{path}").parse().unwrap()
			},

			_ => self.clone().into(),
		}
	}
}
