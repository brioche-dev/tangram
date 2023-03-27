use super::{Component, Template};
use crate::{
	artifact,
	error::{Error, Result, WrapErr},
	path::Path,
	util::fs,
	Instance,
};

impl Instance {
	#[allow(clippy::unused_async)]
	pub async fn unrender(&self, path: fs::PathBuf) -> Result<Template> {
		let template = if let Ok(path) = path.strip_prefix(self.checkouts_path()) {
			// Parse the string as a path.
			let target: Path = path
				.to_str()
				.wrap_err("The path is not valid UTF-8.")?
				.parse()
				.wrap_err("The target is not a valid path.")?;

			// Get the path components.
			let mut components = target.components.iter().peekable();

			// Parse the first component as an artifact hash.
			let artifact_hash: artifact::Hash = components
				.next()
				.wrap_err("Invalid symlink.")?
				.as_str()
				.parse()
				.map_err(Error::other)
				.wrap_err("Failed to parse the path component as a hash.")?;

			// Collect the remaining components to get the path within the referenced artifact.
			let path: Option<Path> = if components.peek().is_some() {
				Some(components.cloned().collect())
			} else {
				None
			};

			// Create the components
			let mut components = Vec::new();
			components.push(Component::String(String::new()));
			components.push(Component::Artifact(artifact_hash));
			if let Some(path) = path {
				components.push(Component::String(format!("/{path}")));
			}

			// Create the template.
			Template { components }
		} else {
			// Convert the path to a string.
			let path = path
				.to_str()
				.wrap_err("The path is not valid UTF-8.")?
				.to_owned();

			// Create the components.
			let components = vec![Component::String(path)];

			// Create the template.
			Template { components }
		};

		Ok(template)
	}
}
