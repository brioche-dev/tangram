use super::Directory;
use crate::{
	artifact::{self, Artifact},
	error::Result,
	instance::Instance,
};
use std::collections::BTreeMap;

impl Directory {
	pub fn new(tg: &Instance, entries: &BTreeMap<String, Artifact>) -> Result<Self> {
		// Get the hashes of the entries.
		let entries: BTreeMap<String, artifact::Hash> = entries
			.iter()
			.map(|(name, artifact)| (name.clone(), artifact.hash()))
			.collect();

		// Create the artifact data.
		let data = artifact::Data::Directory(super::Data {
			entries: entries.clone(),
		});

		// Serialize and hash the artifact data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = artifact::Hash(crate::hash::Hash::new(&bytes));

		// Add the artifact to the database.
		let hash = tg.database.add_artifact(hash, &bytes)?;

		// Create the directory.
		let directory = Self { hash, entries };

		Ok(directory)
	}
}
