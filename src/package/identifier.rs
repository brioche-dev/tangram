use crate::{artifact, os};

/// A unique identifier for a package.
pub enum Identifier {
	/// A package at a path.
	Path(os::PathBuf),

	// A checked in package with a hash.
	Hash(artifact::Hash),
}
