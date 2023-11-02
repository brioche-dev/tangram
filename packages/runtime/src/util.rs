use std::path::Path;
use tangram_client as tg;
use tg::{template, Artifact, Client, Result, Value};

/// Render a value.
pub async fn render(value: &Value, client: &dyn Client, artifacts_path: &Path) -> Result<String> {
	if let Ok(string) = value.try_unwrap_string_ref() {
		Ok(string.clone())
	} else if let Ok(artifact) = Artifact::try_from(value.clone()) {
		Ok(artifacts_path
			.join(artifact.id(client).await?.to_string())
			.into_os_string()
			.into_string()
			.unwrap())
	} else if let Ok(template) = value.try_unwrap_template_ref() {
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
	} else {
		Ok("<tangram value>".to_owned())
	}
}
