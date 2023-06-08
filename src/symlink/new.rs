use super::Symlink;
use crate::{artifact, error::Result, instance::Instance, template::Template};

impl Symlink {
	pub fn new(tg: &Instance, target: Template) -> Result<Self> {
		// Create the artifact data.
		let data = artifact::Data::Symlink(super::Data {
			target: target.to_data(),
		});

		// Serialize and hash the artifact data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = artifact::Hash(crate::hash::Hash::new(&bytes));

		// Add the artifact to the database.
		let hash = tg.database.add_artifact(hash, &bytes)?;

		// Create the symlink.
		let symlink = Self { hash, target };

		Ok(symlink)
	}
}
