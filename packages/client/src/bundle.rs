use crate::{directory, return_error, Artifact, Client, Directory, Error, File, Result, Symlink};
use async_recursion::async_recursion;
use futures::{stream::FuturesOrdered, TryStreamExt};
use once_cell::sync::Lazy;
use tangram_error::WrapErr;

static TANGRAM_ARTIFACTS_PATH: Lazy<crate::Path> =
	Lazy::new(|| ".tangram/artifacts".parse().unwrap());

static TANGRAM_RUN_SUBPATH: Lazy<crate::Path> = Lazy::new(|| ".tangram/run".parse().unwrap());

impl Artifact {
	/// Bundle an artifact with all of its recursive references at `.tangram/artifacts`.
	pub async fn bundle(&self, client: &dyn Client) -> Result<Artifact> {
		// Collect the artifact's recursive references.
		let references = self.recursive_references(client).await?;

		// If there are no references, then return the artifact.
		if references.is_empty() {
			return Ok(self.clone());
		}

		// Create the artifacts directory by removing all references from the referenced artifacts.
		let entries = references
			.into_iter()
			.map(|id| async move {
				let artifact = Artifact::with_id(id.clone());
				let artifact = artifact.remove_references(client, 3).await?;
				Ok::<_, Error>((id.to_string(), artifact))
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
			.remove_references(client, 0)
			.await?
			.try_unwrap_directory()
			.ok()
			.wrap_err("The artifact must be a directory.")?;

		// Add the artifacts directory to the bundled artifact at `.tangram/artifacts`.
		let bundle_directory = bundle_directory
			.builder(client)
			.await?
			.add(client, &TANGRAM_ARTIFACTS_PATH, artifacts_directory.into())
			.await?
			.build()
			.into();

		Ok(bundle_directory)
	}

	/// Remove all references from an artifact and its children recursively.
	#[async_recursion]
	async fn remove_references(
		&self,
		client: &'async_recursion dyn Client,
		depth: usize,
	) -> Result<Artifact> {
		match self {
			// If the artifact is a directory, then recurse to remove references from its entries.
			Artifact::Directory(directory) => {
				let entries = directory
					.entries(client)
					.await?
					.iter()
					.map(|(name, artifact)| async move {
						let artifact = artifact.remove_references(client, depth + 1).await?;
						Ok::<_, Error>((name.clone(), artifact))
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

			// If the artifact is a symlink, then replace it with a symlink pointing to `.tangram/artifacts/<id>`.
			Artifact::Symlink(symlink) => {
				// Render the target.
				let mut target = String::new();
				let artifact = symlink.artifact(client).await?;
				let path = symlink.path(client).await?;
				if let Some(artifact) = artifact {
					for _ in 0..depth {
						target.push_str("../");
					}
					target.push_str(
						&TANGRAM_ARTIFACTS_PATH
							.clone()
							.join(artifact.id(client).await?.to_string().parse().unwrap())
							.to_string(),
					);
				}
				if artifact.is_some() && path.is_some() {
					target.push('/');
				}
				if let Some(path) = path {
					target.push_str(path);
				}
				Ok(Symlink::new(None, Some(target)).into())
			},
		}
	}
}
