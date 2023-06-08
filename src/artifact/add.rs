use super::{Artifact, Data, Hash};
use crate::{error::Result, instance::Instance};

impl Artifact {
	pub async fn add(tg: &Instance, data: Data) -> Result<Self> {
		// Serialize and hash the artifact data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = Hash(crate::hash::Hash::new(&bytes));

		// Add the artifact to the database.
		let hash = tg.database.add_artifact(hash, &bytes)?;

		// Create the artifact.
		let artifact = Self::from_data(tg, hash, data).await?;

		Ok(artifact)
	}
}
