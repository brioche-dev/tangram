use crate::{
	directory, return_error, subpath::Subpath, template, Artifact, Client, Directory, Error, File,
	Result, Symlink, WrapErr,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesOrdered, TryStreamExt};
use once_cell::sync::Lazy;

static TANGRAM_ARTIFACTS_PATH: Lazy<Subpath> = Lazy::new(|| ".tangram/artifacts".parse().unwrap());

static TANGRAM_RUN_SUBPATH: Lazy<Subpath> = Lazy::new(|| ".tangram/run".parse().unwrap());

impl Artifact {
	/// Bundle an artifact with all of its recursive references at `.tangram/artifacts`.
	pub async fn bundle(&self, client: &Client) -> Result<Artifact> {
		// Collect the artifact's recursive references.
		let references = self.recursive_references(client).await?;

		// If there are no references, then return the artifact.
		if references.is_empty() {
			return Ok(self.clone());
		}

		// Create the artifacts directory by removing all references from the referenced artifacts.
		let entries = references
			.into_iter()
			.map(|id| {
				async move {
					let artifact = Artifact::with_id(id);

					// Create the path for the reference at `.tangram/artifacts/<id>`.
					let path = TANGRAM_ARTIFACTS_PATH
						.clone()
						.join(id.to_string().parse().unwrap());

					// Remove references from the referenced artifact.
					let artifact = artifact.remove_references(client, &path).await?;

					Ok::<_, Error>((id.to_string(), artifact))
				}
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;
		let artifacts_directory = Directory::new(entries);

		// Create the bundle directory.
		let bundle_directory: Artifact = match self {
			// If the artifact is a directory, use it as is.
			Artifact::Directory(directory) => directory.clone().into(),

			// If the artifact is an executable file, create a directory and place the executable at `.tangram/run`.
			Artifact::File(file) if file.executable(client).await? => directory::Builder::default()
				.add(client, &TANGRAM_RUN_SUBPATH, file.clone().into())
				.await?
				.build()
				.into(),

			// Otherwise, return an error.
			_ => return_error!("The artifact must be a directory or an executable file."),
		};

		// Remove references from the bundle directory.
		let bundle_directory = bundle_directory
			.remove_references(client, &Subpath::empty())
			.await?;

		// Add the artifacts directory to the bundled artifact at `.tangram/artifacts`.
		let bundle_directory = bundle_directory
			.as_directory()
			.wrap_err("The artifact must be a directory.")?
			.builder(client)
			.await?
			.add(client, &TANGRAM_ARTIFACTS_PATH, artifacts_directory.into())
			.await?
			.build()
			.into();

		Ok(bundle_directory)
	}

	/// Remove all references from an artifact and its children, rendering symlink targets to a relative path from `path` to `.tangram/artifacts/<id>`.
	#[async_recursion]
	async fn remove_references(
		&self,
		client: &'async_recursion Client,
		path: &Subpath,
	) -> Result<Artifact> {
		match self {
			// If the artifact is a directory, then recurse to remove references from its entries.
			Artifact::Directory(directory) => {
				let entries = directory
					.entries(client)
					.await?
					.iter()
					.map(|(name, artifact)| {
						async move {
							// Create the path for the entry.
							let path = path.clone().join(name.parse().unwrap());

							// Remove references from the entry.
							let artifact = artifact.remove_references(client, &path).await?;

							Ok::<_, Error>((name.clone(), artifact))
						}
					})
					.collect::<FuturesOrdered<_>>()
					.try_collect()
					.await?;

				Ok(Directory::new(entries).into())
			},

			// If the artifact is a file, then return the file without any references.
			Artifact::File(file) => Ok(File::new(
				file.contents(client).await?.clone(),
				file.executable(client).await?,
				vec![],
			)
			.into()),

			// If the artifact is a symlink, then return the symlink with its target rendered with artifacts pointing to `.tangram/artifacts/<id>`.
			Artifact::Symlink(symlink) => {
				// Render the target.
				let target = symlink
					.target(client)
					.await?
					.try_render(|component| async move {
						match component {
							// Render a string component as is.
							template::Component::String(string) => Ok(string.into()),

							// Render an artifact component with the diff from the path's parent to the referenced artifact's path.
							template::Component::Artifact(artifact) => {
								let artifact_path = TANGRAM_ARTIFACTS_PATH
									.clone()
									.join(artifact.id(client).await?.to_string().parse().unwrap());
								let path = artifact_path
									.into_relpath()
									.diff(&path.clone().into_relpath().parent())
									.to_string()
									.into();
								Ok(path)
							},

							// Placeholder components are not allowed.
							template::Component::Placeholder(_) => {
								return_error!(
									"Cannot remove references from a symlink whose target has placeholders."
								);
							},
						}
					})
					.await?
					.into();

				Ok(Symlink::new(target).into())
			},
		}
	}
}
