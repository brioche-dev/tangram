use crate::{module, os};
use anyhow::Result;
use url::Url;

impl module::Identifier {
	pub async fn from_lsp_uri(url: Url) -> Result<module::Identifier> {
		match url.scheme() {
			"file" => module::Identifier::for_path(os::Path::new(url.path())).await,
			_ => url.try_into(),
		}
	}

	#[must_use]
	pub fn to_lsp_uri(&self) -> Url {
		match self {
			module::Identifier::Artifact(module::identifier::Artifact {
				source: module::identifier::Source::Path(package_path),
				path,
			})
			| module::Identifier::Normal(module::identifier::Normal {
				source: module::identifier::Source::Path(package_path),
				path,
			}) => {
				let path = package_path.join(path.to_string());
				let path = path.display();
				format!("file://{path}").parse().unwrap()
			},

			_ => self.clone().into(),
		}
	}
}
