use crate::{artifact, util::fs, Instance};

impl Instance {
	/// Add a package tracker.
	pub async fn add_package_tracker(&self, package_hash: artifact::Hash, path: fs::PathBuf) {
		self.package_trackers
			.write()
			.await
			.insert(package_hash, path);
	}

	pub async fn get_package_tracker(&self, package_hash: &artifact::Hash) -> Option<fs::PathBuf> {
		self.package_trackers
			.read()
			.await
			.get(package_hash)
			.cloned()
	}

	/// Remove a module tracker.
	pub async fn remove_package_tracker(&self, package_hash: artifact::Hash) {
		self.package_trackers.write().await.remove(&package_hash);
	}
}
