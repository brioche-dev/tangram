use crate::{lock, Artifact, Client, Dependency, Lock};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tangram_error::{error, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
	pub paths: BTreeMap<crate::Path, lock::Id>,
	pub locks: BTreeMap<lock::Id, BTreeMap<Dependency, lock::data::Entry>>,
}

impl Lockfile {
	/// Recursively create a [`Lockfile`] from an iterator of `(Path, Lock)`.
	pub async fn with_paths(
		client: &dyn Client,
		paths_: impl IntoIterator<Item = (crate::Path, Lock)>,
	) -> Result<Self> {
		let mut paths = BTreeMap::new();
		let mut locks = BTreeMap::new();
		let mut queue = VecDeque::new();
		let mut visited = BTreeSet::new();

		// Create the paths.
		for (relpath, lock) in paths_ {
			let id = lock.id(client).await?.clone();
			paths.insert(relpath, id);
			queue.push_back(lock);
		}

		// Create the locks.
		while let Some(next) = queue.pop_front() {
			let id = next.id(client).await?;
			if visited.contains(id) {
				continue;
			}
			visited.insert(id.clone());
			let mut entry_ = BTreeMap::new();
			for (dependency, entry) in next.dependencies(client).await? {
				queue.push_back(entry.lock.clone());
				let entry = lock::data::Entry {
					package: entry.package.id(client).await?.clone(),
					lock: entry.lock.id(client).await?.clone(),
				};
				entry_.insert(dependency.clone(), entry.clone());
			}
			locks.insert(id.clone(), entry_);
		}

		// Return the lockfile.
		Ok(Self { paths, locks })
	}

	pub fn lock(&self, relpath: &crate::Path) -> Result<Lock> {
		let root = self
			.paths
			.get(relpath)
			.ok_or(error!("No lock exists for path."))?;
		self.create_lock_inner(root)
	}

	fn create_lock_inner(&self, id: &lock::Id) -> Result<Lock> {
		// Lookup the entry.
		let entry = self
			.locks
			.get(id)
			.ok_or(error!("Missing lock in lockfile."))?;

		// Create the dependencies.
		let mut dependencies = BTreeMap::new();
		for (dependency, entry) in entry {
			let package = Artifact::with_id(entry.package.clone());
			let lock = self.create_lock_inner(&entry.lock)?;
			let entry = lock::Entry { package, lock };
			dependencies.insert(dependency.clone(), entry);
		}
		let object = lock::Object { dependencies };
		Ok(Lock::with_object(object))
	}
}
