use crate::server::Server;
use crate::value::Value;
use anyhow::{bail, Result};
use std::sync::Arc;

impl Server {
	pub async fn evaluate_path(
		self: &Arc<Self>,
		path: crate::expression::Path,
	) -> Result<crate::value::Value> {
		let value = self.evaluate(*path.artifact).await?;
		let artifact = match value {
			Value::Artifact(artifact) => artifact,
			_ => bail!("Value is not an artifact."),
		};
		let value = Value::Path(crate::value::Path {
			artifact,
			path: path.path,
		});
		Ok(value)
	}
}
