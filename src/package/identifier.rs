use crate::{artifact, os};

/// A unique identifier for a package, either at a path or with a hash.
#[derive(Clone, Debug)]
pub enum Identifier {
	/// A package at a path.
	Path(os::PathBuf),

	// A package with a hash.
	Hash(artifact::Hash),
}
