use crate::{os, Cli};
use anyhow::Result;
use async_recursion::async_recursion;

impl Cli {
	/// Create a lockfile for the specified package.
	#[async_recursion]
	#[must_use]
	pub async fn create_lockfile(&self, _path: &os::Path) -> Result<()> {
		todo!()
	}
}
