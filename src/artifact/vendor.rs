use super::Hash;
use crate::{error::Result, Instance};

impl Instance {
	// TODO: Implement this.
	pub async fn vendor(&self, artifact_hash: Hash) -> Result<Hash> {
		Ok(artifact_hash)
	}
}
