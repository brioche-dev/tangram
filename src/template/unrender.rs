use super::{Component, Template};
use crate::{artifact::Artifact, error::Result, instance::Instance, util::fs};

impl Template {
	#[allow(clippy::unused_async)]
	pub async fn unrender(tg: &Instance, path: &fs::Path, string: &str) -> Result<Template> {
		// Create the regex.
		let path = path.display().to_string();
		let regex = format!(r"{path}/([0-9a-f]{{64}})");
		let regex = regex::Regex::new(&regex).unwrap();

		let mut i = 0;
		let mut components = vec![];
		for captures in regex.captures_iter(string) {
			// Add the text leading up to the capture as a string component.
			let match_ = captures.get(0).unwrap();
			if match_.start() > i {
				components.push(Component::String(string[i..match_.start()].to_owned()));
			}

			// Get and parse the artifact hash.
			let artifact_hash_match = captures.get(1).unwrap();
			let artifact_hash = artifact_hash_match.as_str().parse().unwrap();

			// Get the artifact.
			let artifact = Artifact::get(tg, artifact_hash).await?;

			// Add an artifact component.
			components.push(Component::Artifact(artifact));

			// Advance the cursor to the end of the match.
			i = match_.end();
		}

		// Add the remaining text as a string component.
		if i < string.len() {
			components.push(Component::String(string[i..].to_owned()));
		}

		// Create the template.
		let template = Template::new(components);

		Ok(template)
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		artifact::Artifact,
		blob::Blob,
		error::Result,
		file::File,
		instance::{Instance, Options},
		template::{self, Template},
	};
	use std::sync::Arc;
	use tempfile::TempDir;

	#[tokio::test]
	async fn test_unrender_artifact_path() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Arc::new(Instance::new(temp_path, Options::default()).await?);

		let artifact: Artifact = File::builder(Blob::new(&tg, "foo".as_bytes()).await?)
			.build(&tg)
			.await?
			.into();
		let artifact_path = artifact
			.check_out_internal(&tg)
			.await?
			.display()
			.to_string();

		let string = artifact_path;

		let left = Template::unrender(&tg, &tg.artifacts_path(), &string).await?;
		let right = template::Template::new(template::Component::Artifact(artifact));
		assert_eq!(left, right);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_artifact_subpath() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Arc::new(Instance::new(temp_path, Options::default()).await?);

		let artifact: Artifact = File::builder(Blob::new(&tg, "foo".as_bytes()).await?)
			.build(&tg)
			.await?
			.into();
		let artifact_path = artifact
			.check_out_internal(&tg)
			.await?
			.display()
			.to_string();

		let string = format!("{artifact_path}/fizz/buzz");

		let left = Template::unrender(&tg, &tg.artifacts_path(), &string).await?;
		let right = template::Template::new(vec![
			template::Component::Artifact(artifact),
			template::Component::String("/fizz/buzz".into()),
		]);
		assert_eq!(left, right);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_arbitrary_path() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Arc::new(Instance::new(temp_path, Options::default()).await?);

		let string = "/etc/resolv.conf";

		let left = Template::unrender(&tg, &tg.artifacts_path(), string).await?;
		let right = template::Template::new(template::Component::String("/etc/resolv.conf".into()));
		assert_eq!(left, right);

		Ok(())
	}

	#[tokio::test]
	async fn test_unrender_mixed_paths() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Arc::new(Instance::new(temp_path, Options::default()).await?);

		let artifact: Artifact = File::builder(Blob::new(&tg, "foo".as_bytes()).await?)
			.build(&tg)
			.await?
			.into();
		let artifact_path = artifact
			.check_out_internal(&tg)
			.await?
			.display()
			.to_string();

		let string = format!("foo {artifact_path} bar");

		let left = Template::unrender(&tg, &tg.artifacts_path(), &string).await?;
		let right = template::Template::new(vec![
			template::Component::String("foo ".into()),
			template::Component::Artifact(artifact),
			template::Component::String(" bar".into()),
		]);
		assert_eq!(left, right);

		Ok(())
	}

	#[tokio::test]
	#[allow(clippy::similar_names)]
	async fn test_unrender_command_with_path_like_variable() -> Result<()> {
		let temp_dir = TempDir::new().unwrap();
		let temp_path = temp_dir.path().to_owned();
		let tg = Arc::new(Instance::new(temp_path, Options::default()).await?);

		let foo: Artifact = File::builder(Blob::new(&tg, "foo".as_bytes()).await?)
			.build(&tg)
			.await?
			.into();
		let foo_path = foo.check_out_internal(&tg).await?.display().to_string();

		let bar: Artifact = File::builder(Blob::new(&tg, "bar".as_bytes()).await?)
			.build(&tg)
			.await?
			.into();
		let bar_path = bar.check_out_internal(&tg).await?.display().to_string();

		let baz: Artifact = File::builder(Blob::new(&tg, "baz".as_bytes()).await?)
			.build(&tg)
			.await?
			.into();
		let baz_path = baz.check_out_internal(&tg).await?.display().to_string();

		let string = format!("PATH={foo_path}:{bar_path}:/bin gcc {baz_path}");

		let left = Template::unrender(&tg, &tg.artifacts_path(), &string).await?;
		let right = template::Template::new(vec![
			template::Component::String("PATH=".into()),
			template::Component::Artifact(foo),
			template::Component::String(":".into()),
			template::Component::Artifact(bar),
			template::Component::String(":/bin gcc ".into()),
			template::Component::Artifact(baz),
		]);
		assert_eq!(left, right);

		Ok(())
	}
}
