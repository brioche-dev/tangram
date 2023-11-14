use std::path::PathBuf;

use crate::{Result, Sender, Server};
use lsp_types as lsp;
use tangram_error::return_error;

impl Server {
	pub(crate) async fn handle_did_change_workspace_folders(
		&self,
		sender: Sender,
		params: lsp::DidChangeWorkspaceFoldersParams,
	) -> Result<()> {
		let added = params
			.event
			.added
			.into_iter()
			.map(|folder| folder.uri)
			.collect();
		let removed = params
			.event
			.removed
			.into_iter()
			.map(|folder| folder.uri)
			.collect();
		self.update_workspace_folders(added, removed).await?;
		self.update_diagnostics(&sender).await?;
		Ok(())
	}

	pub(crate) async fn update_workspace_folders(
		&self,
		added: Vec<lsp::Url>,
		removed: Vec<lsp::Url>,
	) -> Result<()> {
		// Get the client and state.
		let client = self.inner.client.as_ref();
		let mut workspace_roots = self.inner.workspace_roots.write().await;

		// Add any new workspace roots.
		for uri in added {
			let package_path = match uri.scheme() {
				"file" => PathBuf::from(uri.path()),
				_ => return_error!("Invalid URI for workspace folder."),
			};
			let module_path = package_path.join(crate::package::ROOT_MODULE_FILE_NAME);
			if module_path.exists() {
				let _ = crate::package::get_or_create(client, &module_path).await?;
			}
			let _ = workspace_roots.insert(package_path);
		}

		// Remove any stale workspace roots.
		for uri in removed {
			let package_path = match uri.scheme() {
				"file" => PathBuf::from(uri.path()),
				_ => return_error!("Invalid URI for workspace folder."),
			};
			let _ = workspace_roots.remove(&package_path);
		}

		Ok(())
	}
}
