use super::File;
use crate::{
	artifact::{self, Artifact},
	blob::Blob,
	error::Result,
	instance::Instance,
};
use itertools::Itertools;

impl File {
	pub fn new(
		tg: &Instance,
		blob: Blob,
		executable: bool,
		references: &[Artifact],
	) -> Result<Self> {
		// Get the references' hashes.
		let references = references
			.iter()
			.map(artifact::Artifact::hash)
			.collect_vec();

		// Create the artifact data.
		let data = artifact::Data::File(super::Data {
			blob_hash: blob.hash(),
			executable,
			references: references.clone(),
		});

		// Serialize and hash the artifact data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = artifact::Hash(crate::hash::Hash::new(&bytes));

		// Add the artifact to the database.
		let hash = tg.database.add_artifact(hash, &bytes)?;

		// Create the file.
		let file = Self {
			hash,
			blob,
			executable,
			references,
		};

		Ok(file)
	}
}
