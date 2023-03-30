use super::{Component, Template};
use crate::{
	artifact,
	error::{Error, Result, WrapErr},
	return_error,
	util::fs,
	Instance,
};

impl Instance {
	#[allow(clippy::unused_async)]
	pub async fn unrender(&self, artifacts_path: &fs::Path, string: &str) -> Result<Template> {
		// Convert the artifacts path to a string.
		let artifacts_path = artifacts_path
			.to_str()
			.wrap_err("Checkouts path is not valid UTF-8.")?;

		// Iterate over each part of the string.
		let mut components = vec![];
		for (i, part) in string.split(artifacts_path).enumerate() {
			// The first part is a literal component, so add it as-is.
			if i == 0 {
				if !part.is_empty() {
					components.push(Component::String(part.to_owned()));
				}
				continue;
			}

			// The artifact path should be a slash followed by an artifact hash, so strip the slash to get the hash.
			let Some(path) = part.strip_prefix('/') else {
				return_error!("Invalid absolute path in template.");
			};

			// Get the artifact hash. We check for the char boundary before calling `.split_at` to avoid panicking on an out-of-bounds index or on multi-byte UTF-8 sequences.
			if !path.is_char_boundary(artifact::HASH_STRING_LENGTH) {
				return_error!("Invalid absolute path in template.");
			}
			let (artifact_hash, rest) = path.split_at(artifact::HASH_STRING_LENGTH);

			// Parse the artifact hash.
			let artifact_hash: artifact::Hash = artifact_hash
				.parse()
				.map_err(Error::other)
				.wrap_err("Invalid path for template.")?;

			// Add an artifact component.
			components.push(Component::Artifact(artifact_hash));

			// Add everything after the artifact component as a literal.
			if !rest.is_empty() {
				components.push(Component::String(rest.to_owned()));
			}
		}

		Ok(components.into_iter().collect())
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		artifact::{self, Artifact},
		error::Result,
		file::File,
		template, Instance, Options,
	};
	use tempfile::TempDir;

	async fn create_test_artifact(tg: &Instance, content: &str) -> (artifact::Hash, String) {
		let artifact = Artifact::File(File::new(tg.add_blob(content.as_bytes()).await.unwrap()));
		let artifact_hash = tg.add_artifact(&artifact).await.unwrap();
		let artifact_path = tg.artifact_path(artifact_hash);
		(artifact_hash, artifact_path.to_str().unwrap().to_owned())
	}

	#[tokio::test]
	async fn test_unrender_artifact_path() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Instance::new(temp_path, Options::default()).await?;

		let (artifact_hash, artifact_path) = create_test_artifact(&tg, "foo").await;

		let template_unrendered = tg.unrender(&tg.artifacts_path(), &artifact_path).await?;
		let template =
			template::Template::from_iter([template::Component::Artifact(artifact_hash)]);
		assert_eq!(template_unrendered, template);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_artifact_subpath() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Instance::new(temp_path, Options::default()).await?;

		let (artifact_hash, artifact_path) = create_test_artifact(&tg, "foo").await;

		let string = format!("{artifact_path}/fizz/buzz");

		let left = tg.unrender(&tg.artifacts_path(), &string).await?;
		let right = template::Template::from_iter([
			template::Component::Artifact(artifact_hash),
			template::Component::String("/fizz/buzz".into()),
		]);
		assert_eq!(left, right);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_arbitrary_path() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Instance::new(temp_path, Options::default()).await?;

		let string = "/etc/resolv.conf";

		let left = tg.unrender(&tg.artifacts_path(), string).await?;
		let right =
			template::Template::from_iter([template::Component::String("/etc/resolv.conf".into())]);
		assert_eq!(left, right);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_mixed_paths() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Instance::new(temp_path, Options::default()).await?;

		let (artifact, artifact_path) = create_test_artifact(&tg, "foo").await;

		let string = format!("foo {artifact_path} bar");

		let left = tg.unrender(&tg.artifacts_path(), &string).await?;
		let right = template::Template::from_iter([
			template::Component::String("foo ".into()),
			template::Component::Artifact(artifact),
			template::Component::String(" bar".into()),
		]);
		assert_eq!(left, right);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_command_with_path_like_variable() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Instance::new(temp_path, Options::default()).await?;

		let (artifact1, artifact1_path) = create_test_artifact(&tg, "foo").await;
		let (artifact2, artifact2_path) = create_test_artifact(&tg, "bar").await;
		let (artifact3, artifact3_path) = create_test_artifact(&tg, "baz").await;

		let string = format!("PATH={artifact1_path}:{artifact2_path}:/bin gcc {artifact3_path}");

		let left = tg.unrender(&tg.artifacts_path(), &string).await?;
		let right = template::Template::from_iter([
			template::Component::String("PATH=".into()),
			template::Component::Artifact(artifact1),
			template::Component::String(":".into()),
			template::Component::Artifact(artifact2),
			template::Component::String(":/bin gcc ".into()),
			template::Component::Artifact(artifact3),
		]);
		assert_eq!(left, right);

		Ok(())
	}
}
