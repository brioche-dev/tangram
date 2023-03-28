use super::{Artifact, Hash};
use crate::{error::Result, Instance};

impl Artifact {
	/// Collect an artifact's recursive references.
	pub fn collect_recursive_references_into(
		&self,
		tg: &Instance,
		references: &mut Vec<Hash>,
	) -> Result<()> {
		match self {
			Artifact::Directory(directory) => {
				for entry_hash in directory.entries.values() {
					let entry = tg.get_artifact_local(*entry_hash)?;
					entry.collect_recursive_references_into(tg, references)?;
				}
			},

			Artifact::File(file) => {
				for reference in &file.references {
					let reference = tg.get_artifact_local(*reference)?;
					reference.collect_recursive_references_into(tg, references);
				}
			},

			Artifact::Symlink(symlink) => {
				symlink
					.target
					.collect_recursive_references_into(tg, references)?;
			},
		};

		Ok(())
	}
}
