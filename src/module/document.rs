use super::Identifier;
use crate::{language::Range, Instance};
use anyhow::{bail, Result};

/// A module that is open and editable.
pub struct Document {
	/// The document's version.
	pub version: i32,
	/// The document's text.
	pub text: String,
}

impl Instance {
	/// Open a document.
	pub async fn open_document(&self, module_identifier: &Identifier, version: i32, text: String) {
		// Create the document.
		let document = Document { version, text };

		// Add the document.
		self.documents
			.write()
			.await
			.insert(module_identifier.clone(), document);
	}

	/// Close a document.
	pub async fn close_document(&self, module_identifier: &Identifier) {
		self.documents.write().await.remove(module_identifier);
	}

	/// Get a document's version.
	pub async fn get_document_version(&self, module_identifier: &Identifier) -> Option<i32> {
		self.documents
			.read()
			.await
			.get(module_identifier)
			.map(|document| document.version)
	}

	/// Get a document's text.
	pub async fn get_document_text(&self, module_identifier: &Identifier) -> Option<String> {
		self.documents
			.read()
			.await
			.get(module_identifier)
			.map(|document| document.text.clone())
	}

	/// Update a document's version and text.
	pub async fn update_document(
		&self,
		identifier: &Identifier,
		version: i32,
		range: Option<Range>,
		text: String,
	) -> Result<()> {
		// Lock the documents.
		let mut documents = self.documents.write().await;

		// Get the document.
		let Some(document) = documents.get_mut(identifier) else {
			bail!(r#"Could not find a document for the module identifier "{identifier}"."#);
		};

		// Convert the range to bytes.
		let range = if let Some(range) = range {
			range.to_byte_range_in_string(&document.text)
		} else {
			0..document.text.len()
		};

		// Replace the text.
		document.text.replace_range(range, &text);

		// Update the version.
		document.version = version;

		Ok(())
	}
}

impl Instance {
	pub async fn get_document_or_module_version(
		&self,
		module_identifier: &Identifier,
	) -> Result<i32> {
		if let Some(version) = self.get_document_version(module_identifier).await {
			return Ok(version);
		}
		let version = self.get_module_version(module_identifier).await?;
		Ok(version)
	}

	pub async fn load_document_or_module(&self, module_identifier: &Identifier) -> Result<String> {
		if let Some(text) = self.get_document_text(module_identifier).await {
			return Ok(text);
		}
		let text = self.load_module(module_identifier).await?;
		Ok(text)
	}
}
