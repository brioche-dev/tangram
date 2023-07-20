pub use self::data::Data;
use crate::{artifact::Artifact, block::Block, template::Template};

mod data;
mod new;
mod resolve;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Symlink {
	/// The symlink's block.
	block: Block,

	/// The symlink's target.
	target: Template,
}

impl Symlink {
	#[must_use]
	pub fn block(&self) -> Block {
		self.block
	}

	#[must_use]
	pub fn target(&self) -> &Template {
		&self.target
	}

	#[must_use]
	pub fn references(&self) -> Vec<Artifact> {
		self.target.artifacts().cloned().collect()
	}
}
