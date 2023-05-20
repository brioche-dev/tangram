use super::Module;
use crate::error::{Error, Result};
use std::rc::Rc;
use swc_common::{FileName, SourceMap};
use swc_ecma_parser::{Parser, StringInput, Syntax, TsConfig};

impl Module {
	pub fn parse(text: String) -> Result<(swc_ecma_ast::Module, Rc<SourceMap>)> {
		// Construct a parser.
		let source_map = Rc::new(SourceMap::default());
		let source_file = source_map.new_source_file(FileName::Anon, text);
		let input = StringInput::from(&*source_file);
		let syntax = Syntax::Typescript(TsConfig::default());
		let mut parser = Parser::new(syntax, input, None);

		// Parse the module from the source text.
		let module = parser.parse_module().map_err(|e| {
			let message = e.kind().msg().to_string();
			Error::message(message)
		})?;

		Ok((module, source_map))
	}
}
