use super::types::{cb_client4, lock_owner4, stateid4, verifier4, NFS4_OTHER_SIZE};
use num::ToPrimitive;
use std::{
	collections::{BTreeMap, HashMap},
	sync::{Arc, Weak},
};
use tangram_client as tg;
use tg::{blob, Directory, File, Symlink};

#[derive(Clone)]
pub struct State {
	pub nodes: BTreeMap<u64, Arc<Node>>,
	pub blob_readers: BTreeMap<stateid4, Arc<tokio::sync::RwLock<blob::Reader>>>,
	pub clients: HashMap<Vec<u8>, ClientData>,
	pub lock_data: (u32, Vec<u32>),
	pub lock_owners: HashMap<lock_owner4, stateid4>,
}

impl Default for State {
	fn default() -> Self {
		let root = Arc::new_cyclic(|root| Node {
			id: 0,
			parent: root.clone(),
			kind: NodeKind::Root {
				children: tokio::sync::RwLock::new(BTreeMap::default()),
			},
		});

		let nodes = [(0, root)].into_iter().collect();

		Self {
			nodes,
			blob_readers: BTreeMap::default(),
			clients: HashMap::new(),
			lock_data: (0, Vec::new()),
			lock_owners: HashMap::new(),
		}
	}
}

impl std::fmt::Debug for State {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::fmt::Debug::fmt(&self.nodes, f)
	}
}

#[derive(Debug)]
pub struct Node {
	pub id: u64,
	pub parent: Weak<Self>,
	pub kind: NodeKind,
}

/// An node's kind.
#[derive(Debug)]
pub enum NodeKind {
	Root {
		children: tokio::sync::RwLock<BTreeMap<String, Arc<Node>>>,
	},
	Directory {
		directory: Directory,
		children: tokio::sync::RwLock<BTreeMap<String, Arc<Node>>>,
	},
	File {
		file: File,
		size: u64,
	},
	Symlink {
		symlink: Symlink,
	},
}

impl Node {
	pub fn depth(self: &Arc<Self>) -> usize {
		if self.id == 0 {
			0
		} else {
			1 + self.parent.upgrade().unwrap().depth()
		}
	}
}

#[derive(Clone)]
pub struct CallbackData {
	pub ident: u32,
	pub client: cb_client4,
}

#[derive(Clone, Debug)]
pub struct ClientData {
	pub server_id: u64,
	pub client_verifier: verifier4,
	pub server_verifier: verifier4,
	pub callback: cb_client4,
	pub callback_ident: u32,
	pub confirmed: bool,
}

impl State {
	pub fn new_client_data(&self) -> (u64, verifier4) {
		let new_id = (self.clients.len() + 1000).to_u64().unwrap();
		(new_id, new_id.to_be_bytes())
	}

	pub fn acquire_lock(&mut self) -> stateid4 {
		let (count, stack) = &mut self.lock_data;
		let other = [0; NFS4_OTHER_SIZE];
		if let Some(seqid) = stack.pop() {
			stateid4 { seqid, other }
		} else {
			let seqid = *count;
			*count += 1;
			stateid4 { seqid, other }
		}
	}

	pub fn release_lock(&mut self, lock_stateid: &stateid4) {
		let (_, stack) = &mut self.lock_data;
		stack.push(lock_stateid.seqid);
	}
}
