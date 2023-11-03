use async_recursion::async_recursion;
use std::collections::BTreeMap;
use tangram_client as tg;

pub struct Lockfile {
	pub root: tg::lock::data::Entry,
	pub entries: BTreeMap<tg::lock::Id, BTreeMap<tg::Dependency, tg::lock::data::Entry>>,
}

impl Lockfile {
	pub async fn from_package(
		client: &dyn tg::Client,
		package: tg::Artifact,
		lock: tg::Lock,
	) -> tg::Result<Self> {
		let mut entries = BTreeMap::new();
		Self::from_lock_inner(package.clone(), lock.clone(), &mut entries).await;
		let package = package.id(client).await?;
		let lock = lock.id(client).await?.clone();
		let root = tg::lock::data::Entry { package, lock };
		Ok(Self { root, entries })
	}

	#[async_recursion]
	async fn from_lock_inner(
		package: tg::Artifact,
		lock: tg::Lock,
		entries: &mut BTreeMap<tg::lock::Id, BTreeMap<tg::Dependency, tg::lock::data::Entry>>,
	) {
		todo!()
	}

	pub fn to_package(&self) -> tg::Result<(tg::Artifact, tg::Lock)> {
		let id = &self.root.lock;
		let package = tg::Artifact::with_id(self.root.package.clone());
		let tg::lock::Entry { package, lock } = self.to_lock_inner(id, package)?;
		Ok((package, lock))
	}

	fn to_lock_inner(
		&self,
		id: &tg::lock::Id,
		package: tg::Artifact,
	) -> tg::Result<tg::lock::Entry> {
		let raw_dependencies = self
			.entries
			.get(id)
			.ok_or(tg::error!("Lockfile is corrupted."))?;

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
