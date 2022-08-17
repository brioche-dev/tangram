use crate::{artifact::Artifact, object::ObjectHash, server::Server};
use anyhow::Result;
use std::sync::Arc;

impl Server {
	// Create an artifact.
	pub async fn create_artifact(self: &Arc<Self>, object_hash: ObjectHash) -> Result<Artifact> {
		self.database_execute(
			r#"
				replace into artifacts (
					object_hash
				) values (
					$1
				)
			"#,
			(object_hash.to_string(),),
		)
		.await?;
		let artifact = Artifact { object_hash };
		Ok(artifact)
	}
}
