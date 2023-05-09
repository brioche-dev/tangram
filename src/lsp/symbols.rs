use super::Server;
use crate::{error::Result, language, module::Module};
use lsp_types as lsp;

impl Server {
	pub async fn symbols(
		&self,
		params: lsp::DocumentSymbolParams,
	) -> Result<Option<lsp::DocumentSymbolResponse>> {
		// Get the module.
		let module = Module::from_lsp(&self.tg, params.text_document.uri).await?;

		// Get the completion entries.
		let symbols = module.symbols(&self.tg).await?;
		let Some(symbols) = symbols else {
			return Ok(None);
		};

		// TODO: Convert the symbols.
		let symbols = symbols.into_iter().map(collect_symbol_tree).collect();

		Ok(Some(lsp::DocumentSymbolResponse::Nested(symbols)))
	}
}

fn collect_symbol_tree(symbol: language::service::symbols::Symbol) -> lsp::DocumentSymbol {
	let language::service::symbols::Symbol {
		name,
		detail,
		kind,
		tags,
		range,
		selection_range,
		children,
	} = symbol;

	let kind = match kind {
		language::service::symbols::Kind::File => lsp::SymbolKind::FILE,
		language::service::symbols::Kind::Module => lsp::SymbolKind::MODULE,
		language::service::symbols::Kind::Namespace => lsp::SymbolKind::NAMESPACE,
		language::service::symbols::Kind::Package => lsp::SymbolKind::PACKAGE,
		language::service::symbols::Kind::Class => lsp::SymbolKind::CLASS,
		language::service::symbols::Kind::Method => lsp::SymbolKind::METHOD,
		language::service::symbols::Kind::Property => lsp::SymbolKind::PROPERTY,
		language::service::symbols::Kind::Field => lsp::SymbolKind::FIELD,
		language::service::symbols::Kind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
		language::service::symbols::Kind::Enum => lsp::SymbolKind::ENUM,
		language::service::symbols::Kind::Interface => lsp::SymbolKind::INTERFACE,
		language::service::symbols::Kind::Function => lsp::SymbolKind::FUNCTION,
		language::service::symbols::Kind::Variable => lsp::SymbolKind::VARIABLE,
		language::service::symbols::Kind::Constant => lsp::SymbolKind::CONSTANT,
		language::service::symbols::Kind::String => lsp::SymbolKind::STRING,
		language::service::symbols::Kind::Number => lsp::SymbolKind::NUMBER,
		language::service::symbols::Kind::Boolean => lsp::SymbolKind::BOOLEAN,
		language::service::symbols::Kind::Array => lsp::SymbolKind::ARRAY,
		language::service::symbols::Kind::Object => lsp::SymbolKind::OBJECT,
		language::service::symbols::Kind::Key => lsp::SymbolKind::KEY,
		language::service::symbols::Kind::Null => lsp::SymbolKind::NULL,
		language::service::symbols::Kind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
		language::service::symbols::Kind::Event => lsp::SymbolKind::EVENT,
		language::service::symbols::Kind::Operator => lsp::SymbolKind::OPERATOR,
		language::service::symbols::Kind::TypeParameter => lsp::SymbolKind::TYPE_PARAMETER,
	};

	let tags = tags
		.into_iter()
		.map(|tag| match tag {
			language::service::symbols::Tag::Deprecated => lsp::SymbolTag::DEPRECATED,
		})
		.collect();

	let children = children.map(|children| children.into_iter().map(collect_symbol_tree).collect());

	let range = lsp::Range {
		start: lsp::Position {
			line: range.start.line,
			character: range.end.character,
		},
		end: lsp::Position {
			line: range.end.line,
			character: range.end.character,
		},
	};

	let selection_range = lsp::Range {
		start: lsp::Position {
			line: selection_range.start.line,
			character: selection_range.end.character,
		},
		end: lsp::Position {
			line: selection_range.end.line,
			character: selection_range.end.character,
		},
	};

	#[allow(deprecated)]
	lsp::DocumentSymbol {
		name,
		detail,
		kind,
		tags: Some(tags),
		range,
		selection_range,
		children,
		deprecated: None,
	}
}
