use std::path::Path;
use tangram_client as tg;
use tg::{template, Artifact, Client, Result, Value, WrapErr};

/// Render a value.
pub async fn render(value: &Value, client: &dyn Client, artifacts_path: &Path) -> Result<String> {
	if let Ok(string) = value.try_unwrap_string_ref() {
		return Ok(string.clone());
	}

	if let Ok(artifact) = Artifact::try_from(value.clone()) {
		return Ok(artifacts_path
			.join(artifact.id(client).await?.to_string())
			.into_os_string()
			.into_string()
			.unwrap());
	}

	if let Ok(template) = value.try_unwrap_template_ref() {
		return template
			.try_render(|component| async move {
				match component {
					template::Component::String(string) => Ok(string.clone()),
					template::Component::Artifact(artifact) => Ok(artifacts_path
						.join(artifact.id(client).await?.to_string())
						.into_os_string()
						.into_string()
						.unwrap()),
				}
			})
			.await;
	}

	// Get the data.
	let data = value.data(client).await?;

	// Render the data.
	let string = serde_json::to_string(&data).wrap_err("Failed to serialize the value.")?;

	Ok(string)
}
