use super::Server;
use anyhow::Result;
use std::sync::Arc;

impl Server {
	pub async fn garbage_collect(self: &Arc<Self>) -> Result<()> {
		// Go through each of the GC roots that are older than GC_TIMEOUT and add all of their transitive dependencies to a set of reachable objects.

		// Go through all objects currently in the store, check if they are in the reachable set, if not remove them.

		// Where do time outs come into play here?
		Ok(())
	}
}
