use crate::{artifact, util::fs};

/// A unique identifier for a package, either at a path or with a hash.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum Identifier {
	/// A package at a path.
	Path(fs::PathBuf),

	// A package with a hash.
	Hash(artifact::Hash),
}
