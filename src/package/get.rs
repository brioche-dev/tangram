use super::{
	lockfile::{self, Lockfile},
	Package, LOCKFILE_FILE_NAME,
};
use crate::{
	artifact::Artifact,
	error::{Error, Result, WrapErr},
	instance::Instance,
};
use async_recursion::async_recursion;
use futures::{stream::FuturesUnordered, TryStreamExt};

impl Package {
	// #[async_recursion]
	// pub async fn with_block(tg: &'async_recursion Instance, block: Block) -> Result<Self> {
	// 	// Get the artifact.
	// 	let artifact = Artifact::with_block(tg, block).await?;

	// 	// Read the lockfile.
	// 	let lockfile = artifact
	// 		.as_directory()
	// 		.wrap_err("Expected the package to be a directory.")?
	// 		.try_get(tg, &LOCKFILE_FILE_NAME.parse().unwrap())
	// 		.await
	// 		.wrap_err("Failed to get the lockfile.")?;
	// 	let dependencies = if let Some(lockfile) = lockfile {
	// 		let lockfile = lockfile
	// 			.as_file()
	// 			.wrap_err("Expected the lockfile to be a file.")?;
	// 		let lockfile = lockfile
	// 			.contents(tg)
	// 			.await?
	// 			.text(tg)
	// 			.await
	// 			.wrap_err("Failed to read the lockfile.")?;
	// 		let lockfile: Lockfile = serde_json::from_str(&lockfile)
	// 			.map_err(Error::other)
	// 			.wrap_err("Failed to parse the lockfile.")?;
	// 		let dependencies = lockfile
	// 			.dependencies
	// 			.into_iter()
	// 			.map(|(dependency, entry)| async move {
	// 				let block = match entry {
	// 					lockfile::Entry::Locked(id) => Block::with_id(id),
	// 					lockfile::Entry::Unlocked { .. } => unimplemented!(),
	// 				};
	// 				Ok::<_, Error>((dependency, block))
	// 			})
	// 			.collect::<FuturesUnordered<_>>()
	// 			.try_collect()
	// 			.await?;
	// 		Some(dependencies)
	// 	} else {
	// 		None
	// 	};

	// 	Ok(Package {
	// 		artifact,
	// 		dependencies,
	// 	})
	// }
}
