use super::{
	compiler::{Diagnostic, DiagnosticCategory, Position, Range},
	Compiler,
};
use crate::js;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tower_lsp::{
	jsonrpc,
	lsp_types::{self as lsp},
};

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

		// Get the completions.
		let completion_info = self
			.compiler
			.completion(url, position.into())
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Convert the completions.
		if let Some(completion_info) = completion_info {
			let completions = completion_info
				.entries
				.into_iter()
				.map(|completion| lsp::CompletionItem {
					label: completion.name,
					..Default::default()
				})
				.collect();
			Ok(Some(lsp::CompletionResponse::Array(completions)))
		} else {
			Ok(None)
		}
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
		let info = self
			.compiler
			.hover(url, position.into())
			.await
			.map_err(|_| jsonrpc::Error::internal_error())?;
		let info_string = info.and_then(|info| {
			info.display_parts.map(|display_parts| {
				display_parts
					.into_iter()
					.map(|part| part.text)
					.collect::<String>()
			})
		});
		Ok(info_string.map(|info_string| lsp::Hover {
			contents: lsp::HoverContents::Scalar(lsp::MarkedString::from_language_code(
				"typescript".into(),
				info_string,
			)),
			range: None,
		}))
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

		// Convert the definitions.
		let location = locations.and_then(|location| location.into_iter().next());
		let location = location.map(|location| {
			let url = match location.url {
				js::Url::PathModule {
					package_path,
					module_path,
				} => {
					let path = package_path.join(module_path);
					format!("file://{}", path.display()).parse().unwrap()
				},
				js::Url::TsLib { .. }
				| js::Url::PackageModule { .. }
				| js::Url::PackageTargets { .. }
				| js::Url::PathTargets { .. } => location.url.into(),
			};
			lsp::GotoDefinitionResponse::Scalar(lsp::Location {
				uri: url,
				range: location.range.into(),
			})
		});
		Ok(location)
	}
}

impl LanguageServer {
	async fn update_diagnostics(&self) {
		// Perform the check.
		let diagnostics = match self.compiler.get_diagnostics().await {
			Ok(diagnostics) => diagnostics,
			Err(error) => {
				self.client
					.log_message(
						lsp::MessageType::ERROR,
						format!("Failed to get diagnostics.\n{error:?}"),
					)
					.await;
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
		let url = js::Url::try_from(params.text_document.uri)
			.map_err(|_| jsonrpc::Error::internal_error())?;

		// Get the contents for this path.
		let contents = self
			.compiler
			.virtual_text_document(url)
			.map_err(|_| jsonrpc::Error::internal_error())?;

		Ok(Some(contents.into()))
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
			severity: Some(value.category.into()),
			..Default::default()
		}
	}
}

impl From<DiagnosticCategory> for lsp::DiagnosticSeverity {
	fn from(value: DiagnosticCategory) -> Self {
		match value {
			DiagnosticCategory::Error => lsp::DiagnosticSeverity::ERROR,
			DiagnosticCategory::Warning => lsp::DiagnosticSeverity::WARNING,
			DiagnosticCategory::Message => lsp::DiagnosticSeverity::INFORMATION,
			DiagnosticCategory::Suggestion => lsp::DiagnosticSeverity::HINT,
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
pub const VIRTUAL_TEXT_DOCUMENT_SCHEME: &str = "tangram";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualTextDocumentParams {
	pub text_document: lsp::TextDocumentIdentifier,
}
