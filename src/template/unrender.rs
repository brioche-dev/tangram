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

		// Split the string using the checkouts path as a separator. For example, given the following string:
		// > "foo /home/tangram/.checkouts/a1b2c3 bar"
		//
		// ...we'll end up with an iterator like this:
		// > ["foo ", "/a1b2c3 bar"]
		//
		// Each "gap" in the iterator should become an artifact component. The first element is always an arbitrary string literal, and every other element should start with a forward slash followed by an artifact hash (and anything after the artifact hash is an extra string literal component). The end result will be the following template components:
		// > [String("foo "), Artifact("a1b2c3"), String(" bar")]
		let string_parts = string.split(artifacts_path);

		// Iterate over each part of the string.
		let mut components = vec![];
		for (i, part) in string_parts.enumerate() {
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
	use tempfile::TempDir;

	use crate::{artifact, error::Result, file, template, Instance, Options};

	struct TestInstance {
		tg: Instance,

		#[allow(dead_code)]
		temp_dir: TempDir,
	}

	impl TestInstance {
		async fn new() -> Self {
			let temp_dir = TempDir::new().expect("Failed to create temporary directory.");
			let tg = Instance::new(temp_dir.path().to_owned(), Options::default())
				.await
				.expect("Failed to create tg instance.");

			Self { tg, temp_dir }
		}

		async fn make_artifact(&self, content: &str) -> artifact::Hash {
			let blob_hash = self
				.tg
				.add_blob(content.as_bytes())
				.await
				.expect("Failed to add blob.");
			let artifact = artifact::Artifact::File(file::File {
				blob_hash,
				executable: false,
				references: vec![],
			});

			self.tg
				.add_artifact(&artifact)
				.await
				.expect("Failed to add artifact.")
		}

		fn artifact_path(&self, artifact: artifact::Hash) -> String {
			self.tg
				.artifacts_path()
				.join(artifact.to_string())
				.to_str()
				.expect("Invalid UTF-8 path.")
				.to_owned()
		}
	}

	#[tokio::test]
	async fn test_unrender_artifact_path() -> Result<()> {
		let test = TestInstance::new().await;
		let artifacts_path = test.tg.artifacts_path();

		let artifact = test.make_artifact("foo").await;
		let artifact_path = test.artifact_path(artifact);

		let template_unrendered = test.tg.unrender(&artifacts_path, &artifact_path).await?;
		let template = template::Template::from_iter([template::Component::Artifact(artifact)]);
		assert_eq!(template_unrendered, template);

		Ok(())
	}
	#[tokio::test]
	async fn test_unrender_artifact_subpath() -> Result<()> {
		let test = TestInstance::new().await;
		let artifacts_path = test.tg.artifacts_path();

		let artifact = test.make_artifact("foo").await;
		let artifact_subpath = format!("{}/fizz/buzz", test.artifact_path(artifact));

		let template_unrendered = test.tg.unrender(&artifacts_path, &artifact_subpath).await?;
		let template = template::Template::from_iter([
			template::Component::Artifact(artifact),
			template::Component::String("/fizz/buzz".into()),
		]);
		assert_eq!(template_unrendered, template);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_arbitrary_path() -> Result<()> {
		let test = TestInstance::new().await;
		let artifacts_path = test.tg.artifacts_path();

		let template_unrendered = test
			.tg
			.unrender(&artifacts_path, "/etc/resolv.conf")
			.await?;
		let template =
			template::Template::from_iter([template::Component::String("/etc/resolv.conf".into())]);
		assert_eq!(template_unrendered, template);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_mixed_paths() -> Result<()> {
		let test = TestInstance::new().await;
		let artifacts_path = test.tg.artifacts_path();

		let artifact = test.make_artifact("foo").await;
		let artifact_path = test.artifact_path(artifact);

		let template_unrendered = test
			.tg
			.unrender(&artifacts_path, &format!("foo {artifact_path} bar"))
			.await?;
		let template = template::Template::from_iter([
			template::Component::String("foo ".into()),
			template::Component::Artifact(artifact),
			template::Component::String(" bar".into()),
		]);
		assert_eq!(template_unrendered, template);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_command_with_path_like_variable() -> Result<()> {
		let test = TestInstance::new().await;
		let artifacts_path = test.tg.artifacts_path();

		let artifact1 = test.make_artifact("foo").await;
		let artifact1_path = test.artifact_path(artifact1);

		let artifact2 = test.make_artifact("bar").await;
		let artifact2_path = test.artifact_path(artifact2);

		let artifact3 = test.make_artifact("baz").await;
		let artifact3_path = test.artifact_path(artifact3);

		let template_unrendered = test
			.tg
			.unrender(
				&artifacts_path,
				&format!("PATH={artifact1_path}:{artifact2_path}:/bin gcc {artifact3_path}"),
			)
			.await?;
		let template = template::Template::from_iter([
			template::Component::String("PATH=".into()),
			template::Component::Artifact(artifact1),
			template::Component::String(":".into()),
			template::Component::Artifact(artifact2),
			template::Component::String(":/bin gcc ".into()),
			template::Component::Artifact(artifact3),
		]);
		assert_eq!(template_unrendered, template);

		Ok(())
	}
}
