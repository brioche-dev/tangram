use async_recursion::async_recursion;
use tangram_error::WrapErr;
use std::collections::BTreeMap;
use tangram_client as tg;
use tangram_error::error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
	pub root: tg::lock::data::Entry,
	pub entries: BTreeMap<tg::lock::Id, BTreeMap<tg::Dependency, tg::lock::data::Entry>>,
}

impl Lockfile {
	pub async fn from_package(
		client: &dyn tg::Client,
		package: tg::Artifact,
		lock: tg::Lock,
	) -> tangram_error::Result<Self> {
		let mut entries = BTreeMap::new();
		Self::from_lock_inner(client, lock.clone(), &mut entries).await?;
		let package = package.id(client).await?;
		let lock = lock.id(client).await?.clone();
		let root = tg::lock::data::Entry { package, lock };
		Ok(Self { root, entries })
	}

	#[async_recursion]
	async fn from_lock_inner(
		client: &dyn tg::Client,
		lock: tg::Lock,
		entries: &mut BTreeMap<tg::lock::Id, BTreeMap<tg::Dependency, tg::lock::data::Entry>>,
	) -> tangram_error::Result<()> {
		// Get the ID and check if we've already visited this lock.
		let id = lock.id(client).await.wrap_err("Failed to get ID")?.clone();
		if entries.contains_key(&id) {
			return Ok(());
		}

		// Add the data to the lockfile.
		let data = lock.data(client).await.wrap_err("Failed to get data.")?;
		entries.insert(id, data.dependencies.clone());

		// Visit any dependencies.
		for entry in lock.object(client).await?.dependencies.values() {
			Self::from_lock_inner(client, entry.lock.clone(), entries).await?;
		}
		Ok(())
	}

	pub fn to_package(&self) -> tangram_error::Result<(tg::Artifact, tg::Lock)> {
		let id = &self.root.lock;
		let package = tg::Artifact::with_id(self.root.package.clone());
		let tg::lock::Entry { package, lock } = self.to_lock_inner(id, package)?;
		Ok((package, lock))
	}

	fn to_lock_inner(
		&self,
		id: &tg::lock::Id,
		package: tg::Artifact,
	) -> tangram_error::Result<tg::lock::Entry> {
		let raw_dependencies = self
			.entries
			.get(id)
			.ok_or(error!("Lockfile is corrupted."))?;

		let mut dependencies = BTreeMap::new();
		for (dependency, entry) in raw_dependencies {
			let package = tg::Artifact::with_id(entry.package.clone());
			let entry = self.to_lock_inner(&entry.lock, package)?;
			let _ = dependencies.insert(dependency.clone(), entry);
		}

		let object = tg::lock::Object { dependencies };

		let lock = tg::Lock::with_object(object);
		let entry = tg::lock::Entry { package, lock };
		Ok(entry)
	}
}
