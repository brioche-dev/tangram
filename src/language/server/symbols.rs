use super::Server;
use crate::{
	error::Result,
	language::{
		self,
		service::symbols::{Kind, Symbol},
	},
	module::Module,
};
use lsp_types as lsp;

impl Server {
	pub async fn symbols(
		&self,
		params: lsp::DocumentSymbolParams,
	) -> Result<Option<lsp::DocumentSymbolResponse>> {
		// Get the module.
		let module = Module::from_lsp(&self.server, params.text_document.uri).await?;

		// Get the document symbols.
		let symbols = module.symbols(&self.server).await?;
		let Some(symbols) = symbols else {
			return Ok(None);
		};

		// Convert the symbols.
		let symbols = symbols.into_iter().map(collect_symbol_tree).collect();

		Ok(Some(lsp::DocumentSymbolResponse::Nested(symbols)))
	}
}

fn collect_symbol_tree(symbol: Symbol) -> lsp::DocumentSymbol {
	let Symbol {
		name,
		detail,
		kind,
		tags,
		range,
		selection_range,
		children,
	} = symbol;

	let kind = match kind {
		Kind::File => lsp::SymbolKind::FILE,
		Kind::Module => lsp::SymbolKind::MODULE,
		Kind::Namespace => lsp::SymbolKind::NAMESPACE,
		Kind::Package => lsp::SymbolKind::PACKAGE,
		Kind::Class => lsp::SymbolKind::CLASS,
		Kind::Method => lsp::SymbolKind::METHOD,
		Kind::Property => lsp::SymbolKind::PROPERTY,
		Kind::Field => lsp::SymbolKind::FIELD,
		Kind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
		Kind::Enum => lsp::SymbolKind::ENUM,
		Kind::Interface => lsp::SymbolKind::INTERFACE,
		Kind::Function => lsp::SymbolKind::FUNCTION,
		Kind::Variable => lsp::SymbolKind::VARIABLE,
		Kind::Constant => lsp::SymbolKind::CONSTANT,
		Kind::String => lsp::SymbolKind::STRING,
		Kind::Number => lsp::SymbolKind::NUMBER,
		Kind::Boolean => lsp::SymbolKind::BOOLEAN,
		Kind::Array => lsp::SymbolKind::ARRAY,
		Kind::Object => lsp::SymbolKind::OBJECT,
		Kind::Key => lsp::SymbolKind::KEY,
		Kind::Null => lsp::SymbolKind::NULL,
		Kind::EnumMember => lsp::SymbolKind::ENUM_MEMBER,
		Kind::Event => lsp::SymbolKind::EVENT,
		Kind::Operator => lsp::SymbolKind::OPERATOR,
		Kind::TypeParameter => lsp::SymbolKind::TYPE_PARAMETER,
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
