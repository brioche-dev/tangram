use super::{
	lockfile::{self, Lockfile},
	Hash, Package, LOCKFILE_FILE_NAME,
};
use crate::{
	artifact::Artifact,
	error::{Error, Result, WrapErr},
	instance::Instance,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};

impl Package {
	#[async_recursion]
	pub async fn get(tg: &'async_recursion Instance, hash: Hash) -> Result<Self> {
		let artifact = Self::try_get(tg, hash)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the package with hash "{hash}"."#))?;
		Ok(artifact)
	}

	pub async fn try_get(tg: &Instance, hash: Hash) -> Result<Option<Self>> {
		// Get the artifact.
		let Some(artifact) = Artifact::try_get(tg, hash).await? else {
			return Ok(None);
		};

		// Read the lockfile.
		let lockfile = artifact
			.as_directory()
			.wrap_err("Expected the package to be a directory.")?
			.try_get(tg, &LOCKFILE_FILE_NAME.parse().unwrap())
			.await
			.wrap_err("Failed to get the lockfile.")?;
		let dependencies = if let Some(lockfile) = lockfile {
			let lockfile = lockfile
				.as_file()
				.wrap_err("Expected the lockfile to be a file.")?;
			let lockfile = lockfile
				.blob()
				.text(tg)
				.await
				.wrap_err("Failed to read the lockfile.")?;
			let lockfile: Lockfile = serde_json::from_str(&lockfile)
				.map_err(Error::other)
				.wrap_err("Failed to parse the lockfile.")?;
			let dependencies = lockfile
				.dependencies
				.into_iter()
				.map(|(dependency, entry)| async move {
					let hash = match entry {
						lockfile::Entry::Locked(hash) => hash,
						lockfile::Entry::Unlocked { .. } => unimplemented!(),
					};
					Ok::<_, Error>((dependency, hash))
				})
				.collect::<FuturesUnordered<_>>()
				.try_collect()
				.await?;
			Some(dependencies)
		} else {
			None
		};

		Ok(Some(Package {
			artifact,
			dependencies,
		}))
	}
}
