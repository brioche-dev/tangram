use crate::nfs::{
	state::{Node, NodeKind},
	types::*,
	Context, Server,
};
use std::{collections::BTreeMap, sync::Arc};
use tangram_client as tg;
use tg::Artifact;

pub type Arg = String;

pub type ResOp = i32;

impl Server {
	#[tracing::instrument(skip(self))]
	pub async fn handle_lookup(&self, ctx: &mut Context, arg: Arg) -> ResOp {
		let Some(fh) = ctx.current_file_handle else {
			return NFS4ERR_NOFILEHANDLE;
		};

		match self.lookup(fh, &arg).await {
			Ok(fh) => {
				ctx.current_file_handle = Some(fh);
				NFS4_OK
			},
			Err(e) => e,
		}
	}

	pub async fn lookup(&self, parent: FileHandle, name: &str) -> Result<FileHandle, i32> {
		let parent_node = self
			.state
			.read()
			.await
			.nodes
			.get(&parent.node)
			.cloned()
			.ok_or(NFS4ERR_NOENT)?;
		let node = self.get_or_create_child_node(parent_node, name).await?;
		let fh = FileHandle { node: node.id };
		Ok(fh)
	}

	// TODO: unify with FUSE implementation.
	pub async fn get_or_create_child_node(
		&self,
		parent_node: Arc<Node>,
		name: &str,
	) -> Result<Arc<Node>, i32> {
		if name == "." {
			return Ok(parent_node);
		}

		if name == ".." {
			let parent_parent_node = parent_node.parent.upgrade().ok_or(NFS4ERR_IO)?;
			return Ok(parent_parent_node);
		}

		match &parent_node.kind {
			NodeKind::Root { children } | NodeKind::Directory { children, .. } => {
				if let Some(child) = children.read().await.get(name).cloned() {
					return Ok(child);
				}
			},
			_ => {
				tracing::error!("Cannot create child on File or Symlink.");
				return Err(NFS4ERR_NOTDIR);
			},
		}

		let child_artifact = match &parent_node.kind {
			NodeKind::Root { .. } => {
				let id = name.parse().map_err(|e| {
					tracing::error!(?e, "Failed to parse artifact ID.");
					NFS4ERR_NOENT
				})?;
				Artifact::with_id(id)
			},

			NodeKind::Directory { directory, .. } => {
				let entries = directory.entries(self.client.as_ref()).await.map_err(|e| {
					tracing::error!(?e, "Failed to get directory entries.");
					NFS4ERR_IO
				})?;
				entries.get(name).ok_or(NFS4ERR_NOENT)?.clone()
			},
			_ => unreachable!(),
		};

		let node_id = self.state.read().await.nodes.len() as u64 + 1000;
		let kind = match child_artifact {
			Artifact::Directory(directory) => {
				let children = tokio::sync::RwLock::new(BTreeMap::default());
				NodeKind::Directory {
					directory,
					children,
				}
			},
			Artifact::File(file) => {
				let contents = file.contents(self.client.as_ref()).await.map_err(|e| {
					tracing::error!(?e, "Failed to get file contents.");
					NFS4ERR_IO
				})?;
				let size = contents.size(self.client.as_ref()).await.map_err(|e| {
					tracing::error!(?e, "Failed to get size of file's contents.");
					NFS4ERR_IO
				})?;
				NodeKind::File { file, size }
			},
			Artifact::Symlink(symlink) => NodeKind::Symlink { symlink },
		};
		let child_node = Node {
			id: node_id,
			parent: Arc::downgrade(&parent_node),
			kind,
		};
		let child_node = Arc::new(child_node);

		// Add the child node to the parent node.
		match &parent_node.kind {
			NodeKind::Root { children } | NodeKind::Directory { children, .. } => {
				children
					.write()
					.await
					.insert(name.to_owned(), child_node.clone());
			},
			_ => unreachable!(),
		}

		// Add the child node to the nodes.
		self.state
			.write()
			.await
			.nodes
			.insert(child_node.id, child_node.clone());

		Ok(child_node)
	}
}
