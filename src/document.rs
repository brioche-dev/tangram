use crate::{
	module::range::Range, package::ROOT_MODULE_FILE_NAME, return_error, subpath::Subpath, Result,
	Server, WrapErr,
};
use std::{
	path::{Path, PathBuf},
	time::SystemTime,
};

/// A document.
#[derive(
	Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "camelCase")]
pub struct Document {
	/// The path to the package.
	pub package_path: PathBuf,

	/// The module path.
	pub module_path: Subpath,
}

/// A document's state.
#[derive(Clone, Debug)]
pub enum State {
	/// A closed document.
	Closed(Closed),

	/// An opened document.
	Opened(Opened),
}

/// A closed document.
#[derive(Clone, Debug)]
pub struct Closed {
	/// The document's version.
	pub version: i32,

	/// The document's last modified time.
	pub modified: SystemTime,
}

/// An opened document.
#[derive(Clone, Debug)]
pub struct Opened {
	/// The document's version.
	pub version: i32,

	/// The document's text.
	pub text: String,
}

impl Document {
	pub async fn new(server: &Server, package_path: PathBuf, module_path: Subpath) -> Result<Self> {
		let path = package_path.join(module_path.to_string());

		// Create the document.
		let document = Self {
			package_path,
			module_path,
		};

		// Lock the documents.
		let mut documents = server.state.documents.write().await;

		// Set the state to unopened if it is not present.
		if !documents.contains_key(&document) {
			let metadata = tokio::fs::metadata(&path).await?;
			let modified = metadata.modified()?;
			let state = State::Closed(Closed {
				version: 0,
				modified,
			});
			documents.insert(document.clone(), state);
		}

		Ok(document)
	}

	pub async fn for_path(server: &Server, path: &Path) -> Result<Self> {
		// Find the package path by searching the path's ancestors for a root module.
		let mut found = false;
		let mut package_path = path.to_owned();
		while package_path.pop() {
			if tokio::fs::try_exists(&package_path.join(ROOT_MODULE_FILE_NAME)).await? {
				found = true;
				break;
			}
		}
		if !found {
			let path = path.display();
			return_error!(r#"Could not find the package for path "{path}"."#);
		}

		// Get the module path by stripping the package path.
		let module_path: Subpath = path
			.strip_prefix(&package_path)
			.unwrap()
			.to_owned()
			.into_os_string()
			.into_string()
			.ok()
			.wrap_err("The module path was not valid UTF-8.")?
			.parse()
			.wrap_err("Failed to parse the module path.")?;

		// Create the document.
		let document = Self::new(server, package_path, module_path).await?;

		Ok(document)
	}

	/// Open a document.
	pub async fn open(&self, server: &Server, version: i32, text: String) -> Result<()> {
		// Lock the documents.
		let mut documents = server.state.documents.write().await;

		// Set the state.
		let state = State::Opened(Opened { version, text });
		documents.insert(self.clone(), state);

		Ok(())
	}

	/// Update a document.
	pub async fn update(
		&self,
		server: &Server,
		range: Option<Range>,
		version: i32,
		text: String,
	) -> Result<()> {
		// Lock the documents.
		let mut documents = server.state.documents.write().await;

		// Get the state.
		let Some(State::Opened(state)) = documents.get_mut(self) else {
			let path = self.path();
			let path = path.display();
			return_error!(r#"Could not find an open document for the path "{path}"."#);
		};

		// Update the version.
		state.version = version;

		// Convert the range to bytes.
		let range = if let Some(range) = range {
			range.to_byte_range_in_string(&state.text)
		} else {
			0..state.text.len()
		};

		// Replace the text.
		state.text.replace_range(range, &text);

		Ok(())
	}

	/// Close a document.
	pub async fn close(self, server: &Server) -> Result<()> {
		// Lock the documents.
		let mut documents = server.state.documents.write().await;

		// Remove the document.
		documents.remove(&self);

		Ok(())
	}

	/// Get the document's path.
	#[must_use]
	pub fn path(&self) -> PathBuf {
		self.package_path.join(self.module_path.to_string())
	}

	/// Get the document's version.
	pub async fn version(&self, server: &Server) -> Result<i32> {
		// Lock the documents.
		let mut documents = server.state.documents.write().await;

		// Get the state.
		let state = documents.get_mut(self).unwrap();

		let version = match state {
			State::Closed(closed) => {
				let metadata = tokio::fs::metadata(self.path()).await?;
				let modified = metadata.modified()?;
				if modified > closed.modified {
					closed.modified = modified;
					closed.version += 1;
				}
				closed.version
			},
			State::Opened(opened) => opened.version,
		};

		Ok(version)
	}

	/// Get the document's text.
	pub async fn text(&self, server: &Server) -> Result<String> {
		let path = self.path();
		let documents = server.state.documents.read().await;
		let document = documents.get(self).unwrap();
		let text = match document {
			State::Closed(_) => tokio::fs::read_to_string(&path).await?,
			State::Opened(opened) => opened.text.clone(),
		};
		Ok(text)
	}
}
