pub use self::data::Data;
use crate::{artifact, template::Template};

mod data;
mod new;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Symlink {
	hash: artifact::Hash,
	target: Template,
}

impl Symlink {
	#[must_use]
	pub fn hash(&self) -> artifact::Hash {
		self.hash
	}

	#[must_use]
	pub fn target(&self) -> &Template {
		&self.target
	}
}
