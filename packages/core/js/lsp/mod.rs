use super::{
	compiler::{Diagnostic, Position, Range},
	Compiler,
};
use crate::js;
use async_trait::async_trait;
use std::path::Path;
use tower_lsp::{jsonrpc, lsp_types as lsp};

pub struct LanguageServer {
	client: tower_lsp::Client,
	compiler: Compiler,
}

impl LanguageServer {
	#[must_use]
	pub fn new(client: tower_lsp::Client, compiler: Compiler) -> LanguageServer {
		LanguageServer { client, compiler }
	}
}

#[async_trait]
impl tower_lsp::LanguageServer for LanguageServer {
	async fn initialize(
		&self,
		_params: lsp::InitializeParams,
	) -> jsonrpc::Result<lsp::InitializeResult> {
		Ok(lsp::InitializeResult {
			capabilities: lsp::ServerCapabilities {
				text_document_sync: Some(lsp::TextDocumentSyncCapability::Kind(
					lsp::TextDocumentSyncKind::FULL,
				)),
				..Default::default()
			},
			..Default::default()
		})
	}

	async fn shutdown(&self) -> jsonrpc::Result<()> {
		Ok(())
	}

	async fn did_open(&self, params: lsp::DidOpenTextDocumentParams) {
		// Get the document info.
		let path = Path::new(params.text_document.uri.path());
		let version = params.text_document.version;
		let source = params.text_document.text;

		// Open the document with the compiler.
		self.compiler.open_document(path, version, source).await;

		// Update all diagnostics.
		self.check_documents().await;
	}

	async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
		// Get the document's path.
		let path = Path::new(params.text_document.uri.path());

		// Update the document in the compiler.
		for change in params.content_changes {
			self.compiler
				.change_document(path, params.text_document.version, change.text)
				.await;
		}

		// Update all diagnostics.
		self.check_documents().await;
	}

	async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
		// Get the document's path.
		let path = Path::new(params.text_document.uri.path());

		// Close the document in the compiler.
		self.compiler.close_document(path).await;

		// Update all diagnostics.
		self.check_documents().await;
	}
}

impl LanguageServer {
	async fn check_documents(&self) {
		// Perform the check.
		let diagnostics = if let Ok(diagnostics) = dbg!(self.compiler.get_diagnostics().await) {
			diagnostics
		} else {
			return;
		};

		// Publish the diagnostics.
		for (url, diagnostics) in diagnostics {
			let path = match url {
				js::Url::PathModule {
					package_path,
					sub_path,
				} => package_path.join(sub_path),
				_ => continue,
			};
			let url = format!("file://{}", path.display()).parse().unwrap();
			let diagnostics = diagnostics.into_iter().map(Into::into).collect();
			self.client
				.publish_diagnostics(url, diagnostics, None)
				.await;
		}
	}
}

impl From<Diagnostic> for lsp::Diagnostic {
	fn from(value: Diagnostic) -> Self {
		lsp::Diagnostic {
			message: value.message,
			range: value
				.location
				.map(|location| location.range.into())
				.unwrap_or_default(),
			..Default::default()
		}
	}
}

impl From<Range> for lsp::Range {
	fn from(value: Range) -> Self {
		lsp::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Range> for Range {
	fn from(value: lsp::Range) -> Self {
		Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Position> for Position {
	fn from(value: lsp::Position) -> Self {
		Position {
			line: value.line,
			character: value.character,
		}
	}
}

impl From<Position> for lsp::Position {
	fn from(value: Position) -> Self {
		lsp::Position {
			line: value.line,
			character: value.character,
		}
	}
}
