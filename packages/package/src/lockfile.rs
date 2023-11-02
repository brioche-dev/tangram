pub struct Lockfile {
	root: tg::lock::Id,
	entries: BTreeMap<tg::lock::Id, BTreeMap<Dependency, tg::lock::data::Entry>>,
}
