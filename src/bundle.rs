use crate::{
	self as tg,
	artifact::Artifact,
	directory,
	error::{return_error, Error, Result, WrapErr},
	server::Server,
	subpath::Subpath,
	template,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesOrdered, TryStreamExt};
use once_cell::sync::Lazy;

static TANGRAM_ARTIFACTS_PATH: Lazy<Subpath> = Lazy::new(|| ".tangram/artifacts".parse().unwrap());
static TANGRAM_RUN_SUBPATH: Lazy<Subpath> = Lazy::new(|| ".tangram/run".parse().unwrap());

impl tg::Artifact {
	/// Bundle an artifact with all of its recursive references at `.tangram/artifacts`.
	pub async fn bundle(&self, tg: &Server) -> Result<tg::Artifact> {
		// Collect the artifact's recursive references.
		let references = self.recursive_references(tg).await?;

		// If there are no references, then return the artifact.
		if references.is_empty() {
			return Ok(self.clone());
		}

		// Create the bundle directory
		let artifact = match self.get() {
			// If the artifact is a directory, use it as is.
			Artifact::Directory(directory) => Artifact::Directory(directory.clone()),

			// If the artifact is an executable file, create a directory and place the executable at `.tangram/run`.
			Artifact::File(file) if file.executable(tg).await? => directory::Builder::default()
				.add(tg, &TANGRAM_RUN_SUBPATH, file.clone())
				.await?
				.build()
				.into(),

			// Otherwise, return an error.
			_ => return_error!("The artifact must be a directory or an executable file."),
		};

		// Bundle the artifact, stripping any references recursively.
		let artifact = artifact.bundle_inner(tg, &Subpath::empty()).await?;

		// Create the references directory by bundling each reference at `.tangram/artifacts/<id>`.
		let entries = references
			.into_iter()
			.map(|reference| {
				async move {
					// Create the path for the reference at `.tangram/artifacts/<id>`.
					let path = TANGRAM_ARTIFACTS_PATH
						.clone()
						.join(reference.id().to_string().parse().unwrap());

					// Bundle the reference.
					let artifact = reference.bundle_inner(tg, &path).await?;

					Ok::<_, Error>((reference.id().to_string(), artifact))
				}
			})
			.collect::<FuturesOrdered<_>>()
			.try_collect()
			.await?;

		let directory = tg::Directory::new(&entries);

		// Add the references directory to the artifact at `.tangram/artifacts`.
		let artifact = artifact
			.into_directory()
			.wrap_err("The artifact must be a directory.")?
			.builder(tg)
			.await?
			.add(tg, &TANGRAM_ARTIFACTS_PATH, directory)
			.await?
			.build()
			.into();

		Ok(artifact)
	}

	/// Remove all references from an artifact recursively, rendering symlink targets to a relative path from `path` to `.tangram/artifacts/<id>`.
	#[async_recursion]
	async fn bundle_inner(&self, tg: &'async_recursion Server, path: &Subpath) -> Result<Artifact> {
		match self.get() {
			// If the artifact is a directory, then recurse to bundle its entries.
			Artifact::Directory(directory) => {
				let entries = directory
					.entries(tg)
					.await?
					.into_iter()
					.map(|(name, artifact)| {
						async move {
							// Create the path for the entry.
							let path = path.clone().join(name.parse().unwrap());

							// Bundle the entry.
							let artifact = artifact.bundle_inner(tg, &path).await?;

							Ok::<_, Error>((name, artifact))
						}
					})
					.collect::<FuturesOrdered<_>>()
					.try_collect()
					.await?;

				Ok(Artifact::Directory(tg::Directory::new(&entries)))
			},

			// If the artifact is a file, then return the file without any references.
			Artifact::File(file) => Ok(Artifact::File(File::new(
				file.contents(tg).await?,
				file.executable(),
				vec![],
			))),

			// If the artifact is a symlink, then return the symlink with its target rendered with artifacts pointing to `.tangram/artifacts/<id>`.
			Artifact::Symlink(symlink) => {
				// Render the target.
				let target = symlink
					.target(tg)
					.await?
					.get(tg)
					.await?
					.try_render(|component| async move {
						match component {
							// Render a string component as is.
							template::Component::String(string) => Ok(string.into()),

							// Render an artifact component with the diff from the path's parent to the referenced artifact's bundled path.
							template::Component::Artifact(artifact) => {
								let artifact_path = TANGRAM_ARTIFACTS_PATH
									.clone()
									.join(artifact.id().to_string().parse().unwrap());
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
									"Cannot bundle a symlink whose target has placeholders."
								);
							},
						}
					})
					.await?
					.into();

				Ok(Artifact::Symlink(tg::Symlink::new(target)))
			},
		}
	}
}
