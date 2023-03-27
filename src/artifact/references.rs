use super::{Artifact, Hash};
use crate::{error::Result, Instance};

impl Artifact {
	/// Collect all artifact hashes an artifact references recursively.
	pub fn collect_references(&self, tg: &Instance, references: &mut Vec<Hash>) -> Result<()> {
		match self {
			Artifact::Directory(directory) => {
				for entry_hash in directory.entries.values() {
					let entry = tg.get_artifact_local(*entry_hash)?;
					entry.collect_references(tg, references)?;
				}
			},

			Artifact::File(file) => {
				references.extend(&file.references);
			},

			Artifact::Symlink(symlink) => {
				symlink.target.collect_references(tg, references);
			},
		};

		Ok(())
	}
}
