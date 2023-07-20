use super::{
	lockfile::{self, Lockfile},
	Package, LOCKFILE_FILE_NAME,
};
use crate::{
	artifact::Artifact,
	block::Block,
	error::{Error, Result, WrapErr},
	instance::Instance,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};

impl Package {
	#[async_recursion]
	pub async fn get(tg: &'async_recursion Instance, block: Block) -> Result<Self> {
		let artifact = Self::try_get(tg, block)
			.await?
			.wrap_err_with(|| format!(r#"Failed to get the package with block "{block}"."#))?;
		Ok(artifact)
	}

	pub async fn try_get(tg: &Instance, block: Block) -> Result<Option<Self>> {
		// Get the artifact.
		let Some(artifact) = Artifact::try_get(tg, block).await? else {
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
				.contents(tg)
				.await?
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
					let id = match entry {
						lockfile::Entry::Locked(id) => Block::with_id(id),
						lockfile::Entry::Unlocked { .. } => unimplemented!(),
					};
					Ok::<_, Error>((dependency, id))
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
