use super::{
	compiler::types::{Diagnostic, Position, Range, Severity},
	Compiler,
};
use crate::js;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
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
				hover_provider: Some(lsp::HoverProviderCapability::Simple(true)),
				references_provider: Some(lsp::OneOf::Left(true)),
				completion_provider: Some(lsp::CompletionOptions::default()),
				definition_provider: Some(lsp::OneOf::Left(true)),
				text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
					lsp::TextDocumentSyncOptions {
						open_close: Some(true),
						change: Some(lsp::TextDocumentSyncKind::FULL),
						..Default::default()
					},
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
		// Only proceed if the url has a file scheme.
		let scheme = params.text_document.uri.scheme();
		if scheme != "file" {
			return;
		}

		// Get the file path, version, and text.
		let path = Path::new(params.text_document.uri.path());
		let version = params.text_document.version;
		let text = params.text_document.text;

		// Open the file with the compiler.
		self.compiler.open_file(path, version, text).await;

		// Update all diagnostics.
		self.update_diagnostics().await;
	}

	async fn did_change(&self, params: lsp::DidChangeTextDocumentParams) {
		// Get the file's path.
		let path = Path::new(params.text_document.uri.path());

		// Update the document in the compiler.
		for change in params.content_changes {
			self.compiler
				.change_file(path, params.text_document.version, change.text)
				.await;
		}

		// Update all diagnostics.
		self.update_diagnostics().await;
	}

	async fn did_close(&self, params: lsp::DidCloseTextDocumentParams) {
		// Get the document's path.
		let path = Path::new(params.text_document.uri.path());

		// Close the file in the compiler.
		self.compiler.close_file(path).await;

		// Update all diagnostics.
		self.update_diagnostics().await;
	}

	async fn completion(
		&self,
		params: lsp::CompletionParams,
	) -> jsonrpc::Result<Option<lsp::CompletionResponse>> {
		// Get the position for the request.
		let position = params.text_document_position.position;

		// Parse the path.
		let path: PathBuf = params
			.text_document_position
			.text_document
			.uri
			.path()
			.parse()
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the url for this path.
		let url = js::Url::for_path(&path)
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the completion entries.
		let entries = self
			.compiler
			.completion(url, position.into())
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		let Some(entries) = entries else {
			return Ok(None);
		};

		// Convert the completion entries.
		let entries = entries
			.into_iter()
			.map(|completion| lsp::CompletionItem {
				label: completion.name,
				..Default::default()
			})
			.collect();

		Ok(Some(lsp::CompletionResponse::Array(entries)))
	}

	async fn hover(&self, params: lsp::HoverParams) -> jsonrpc::Result<Option<lsp::Hover>> {
		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Parse the path.
		let path: PathBuf = params
			.text_document_position_params
			.text_document
			.uri
			.path()
			.parse()
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the url for this path.
		let url = js::Url::for_path(&path)
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the hover info.
		let hover = self
			.compiler
			.hover(url, position.into())
			.await
			.map_err(|error| {
				eprintln!("{error:?}");
				jsonrpc::Error::internal_error()
			})?;
		let Some(hover) = hover else {
			return Ok(None);
		};

		// Create the hover.
		let hover = lsp::Hover {
			contents: lsp::HoverContents::Scalar(lsp::MarkedString::from_language_code(
				"typescript".into(),
				hover,
			)),
			range: None,
		};

		Ok(Some(hover))
	}

	async fn goto_definition(
		&self,
		params: lsp::GotoDefinitionParams,
	) -> jsonrpc::Result<Option<lsp::GotoDefinitionResponse>> {
		// Get the position for the request.
		let position = params.text_document_position_params.position;

		// Parse the path.
		let path: PathBuf = params
			.text_document_position_params
			.text_document
			.uri
			.path()
			.parse()
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the url for this path.
		let url = js::Url::for_path(&path)
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the definitions.
		let locations = self
			.compiler
			.goto_definition(url, position.into())
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		let Some(locations) = locations else {
			return Ok(None);
		};

		// Convert the definitions.
		let locations = locations
			.into_iter()
			.map(|location| {
				// Map the URL.
				let url = match location.url {
					js::Url::PathModule {
						package_path,
						module_path,
					} => {
						let path = package_path.join(module_path);
						format!("file://{}", path.display()).parse().unwrap()
					},
					js::Url::Lib { .. }
					| js::Url::PackageModule { .. }
					| js::Url::PackageTargets { .. }
					| js::Url::PathTargets { .. } => location.url.into(),
				};

				lsp::Location {
					uri: url,
					range: location.range.into(),
				}
			})
			.collect();

		let response = lsp::GotoDefinitionResponse::Array(locations);

		Ok(Some(response))
	}

	async fn references(
		&self,
		params: lsp::ReferenceParams,
	) -> jsonrpc::Result<Option<Vec<lsp::Location>>> {
		// Get the position for the request.
		let position = params.text_document_position.position;

		// Parse the path.
		let path: PathBuf = params
			.text_document_position
			.text_document
			.uri
			.path()
			.parse()
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the url for this path.
		let url = js::Url::for_path(&path)
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the references.
		let locations = self
			.compiler
			.get_references(url, position.into())
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		let Some(locations) = locations else {
				return Ok(None);
			};

		// Convert the reference.
		let locations = locations
			.into_iter()
			.map(|location| {
				// Map the URL.
				let url = match location.url {
					js::Url::PathModule {
						package_path,
						module_path,
					} => {
						let path = package_path.join(module_path);
						format!("file://{}", path.display()).parse().unwrap()
					},
					js::Url::Lib { .. }
					| js::Url::PackageModule { .. }
					| js::Url::PackageTargets { .. }
					| js::Url::PathTargets { .. } => location.url.into(),
				};

				lsp::Location {
					uri: url,
					range: location.range.into(),
				}
			})
			.collect();

		Ok(Some(locations))
	}
}

impl LanguageServer {
	async fn update_diagnostics(&self) {
		// Perform the check.
		let diagnostics = match self.compiler.get_diagnostics().await {
			Ok(diagnostics) => diagnostics,
			Err(error) => {
				eprintln!("{error:?}");
				return;
			},
		};

		// Publish the diagnostics.
		for (url, diagnostics) in diagnostics {
			let version = self.compiler.get_version(&url).await.ok();
			let path = match url {
				js::Url::PathModule {
					package_path,
					module_path,
				} => package_path.join(module_path),
				_ => continue,
			};
			let url = format!("file://{}", path.display()).parse().unwrap();
			let diagnostics = diagnostics.into_iter().map(Into::into).collect();
			self.client
				.publish_diagnostics(url, diagnostics, version)
				.await;
		}
	}

	#[allow(clippy::unused_async)]
	pub async fn virtual_text_document(
		&self,
		params: serde_json::Value,
	) -> jsonrpc::Result<Option<serde_json::Value>> {
		// Parse the parameters.
		let params: VirtualTextDocumentParams =
			serde_json::from_value(params).map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the url for the virtual document.
		let url = params
			.text_document
			.uri
			.try_into()
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Load the file.
		let text = self
			.compiler
			.load(&url)
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		Ok(Some(text.into()))
	}
}

impl From<Diagnostic> for lsp::Diagnostic {
	fn from(value: Diagnostic) -> Self {
		let range = value
			.location
			.map(|location| location.range.into())
			.unwrap_or_default();
		let severity = Some(value.severity.into());
		let source = Some("tangram".to_owned());
		let message = value.message;
		lsp::Diagnostic {
			range,
			severity,
			source,
			message,
			..Default::default()
		}
	}
}

impl From<Severity> for lsp::DiagnosticSeverity {
	fn from(value: Severity) -> Self {
		match value {
			Severity::Error => lsp::DiagnosticSeverity::ERROR,
			Severity::Warning => lsp::DiagnosticSeverity::WARNING,
			Severity::Information => lsp::DiagnosticSeverity::INFORMATION,
			Severity::Hint => lsp::DiagnosticSeverity::HINT,
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

pub const VIRTUAL_TEXT_DOCUMENT_REQUEST: &str = "tangram/virtualTextDocument";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualTextDocumentParams {
	pub text_document: lsp::TextDocumentIdentifier,
}
