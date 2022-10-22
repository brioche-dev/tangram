use super::Compiler;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::{jsonrpc, lsp_types as lsp};

pub struct LanguageServer(Arc<RwLock<State>>);

struct State {
	_client: tower_lsp::Client,
	_compiler: Compiler,
}

impl LanguageServer {
	#[must_use]
	pub fn new(client: tower_lsp::Client, compiler: Compiler) -> LanguageServer {
		LanguageServer(Arc::new(RwLock::new(State {
			_client: client,
			_compiler: compiler,
		})))
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
				text_document_sync: Some(lsp::TextDocumentSyncCapability::Options(
					lsp::TextDocumentSyncOptions {
						open_close: Some(false),
						change: None,
						will_save: Some(false),
						will_save_wait_until: None,
						save: Some(lsp::TextDocumentSyncSaveOptions::SaveOptions(
							lsp::SaveOptions {
								include_text: Some(false),
							},
						)),
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
}
