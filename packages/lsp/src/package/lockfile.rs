use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tangram_client as tg;
use tangram_error::{error, Result};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
	pub paths: BTreeMap<tg::Relpath, tg::lock::Id>,
	pub locks: BTreeMap<tg::lock::Id, BTreeMap<tg::Dependency, tg::lock::data::Entry>>,
}

impl Lockfile {
	/// Recursively create a [`Lockfile`] from an iterator of `(Relpath, Lock)`.
	pub async fn with_paths(
		client: &dyn tg::Client,
		paths_: impl IntoIterator<Item = (tg::Relpath, tg::Lock)>,
	) -> Result<Self> {
		let mut paths = BTreeMap::new();
		let mut locks = BTreeMap::new();
		let mut queue = VecDeque::new();
		let mut visited = BTreeSet::new();

		// Create the paths.
		for (relpath, lock) in paths_ {
			let id = lock.id(client).await?.clone();
			let _ = paths.insert(relpath, id);
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
				let entry = tg::lock::data::Entry {
					package: entry.package.id(client).await?.clone(),
					lock: entry.lock.id(client).await?.clone(),
				};
				entry_.insert(dependency.clone(), entry.clone());
			}
			let _ = locks.insert(id.clone(), entry_);
		}

		// Return the lockfile.
		Ok(Self { paths, locks })
	}

	pub fn lock(&self, relpath: &tg::Relpath) -> Result<tg::Lock> {
		let root = self
			.paths
			.get(relpath)
			.ok_or(error!("No lock exists for path."))?;
		self.create_lock_inner(root)
	}

	fn create_lock_inner(&self, id: &tg::lock::Id) -> Result<tg::Lock> {
		// Lookup the entry.
		let entry = self
			.locks
			.get(id)
			.ok_or(error!("Missing lock in lockfile."))?;

		// Create the dependencies.
		let mut dependencies = BTreeMap::new();
		for (dependency, entry) in entry {
			let package = tg::Artifact::with_id(entry.package.clone());
			let lock = self.create_lock_inner(&entry.lock)?;
			let entry = tg::lock::Entry { package, lock };
			dependencies.insert(dependency.clone(), entry);
		}
		let object = tg::lock::Object { dependencies };
		Ok(tg::Lock::with_object(object))
	}
}
