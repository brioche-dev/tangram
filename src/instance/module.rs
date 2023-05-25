use crate::{
	document, hash,
	package::{self, Package},
};
use std::collections::HashMap;

/// State required to support module operations.
pub(crate) struct State {
	/// A map of package specifiers to packages.
	pub(crate) packages: std::sync::RwLock<HashMap<Package, package::Specifier, hash::BuildHasher>>,

	/// A map of paths to documents.
	pub(crate) documents:
		tokio::sync::RwLock<HashMap<document::Document, document::State, fnv::FnvBuildHasher>>,
}

impl State {
	pub(crate) fn new() -> State {
		// Create the packages map.
		let packages = std::sync::RwLock::new(HashMap::default());

		// Create the documents maps.
		let documents = tokio::sync::RwLock::new(HashMap::default());

		Self {
			packages,
			documents,
		}
	}
}
