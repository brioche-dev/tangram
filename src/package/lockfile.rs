use crate::{error::Result, os, Instance};
use async_recursion::async_recursion;

impl Instance {
	/// Create a lockfile for the specified package.
	#[async_recursion]
	#[must_use]
	pub async fn create_lockfile(&self, _path: &os::Path) -> Result<()> {
		todo!()
	}
}
