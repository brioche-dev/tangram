use super::Module;
use crate::{error, Result};
use std::rc::Rc;
use swc_core::{
	common::{FileName, SourceMap},
	ecma::parser::{Parser, StringInput, Syntax, TsConfig},
};

pub struct Output {
	pub module: swc_core::ecma::ast::Module,
	pub source_map: Rc<SourceMap>,
}

impl Module {
	pub fn parse(text: String) -> Result<Output> {
		// Create the parser.
		let source_map = Rc::new(SourceMap::default());
		let source_file = source_map.new_source_file(FileName::Anon, text);
		let input = StringInput::from(&*source_file);
		let syntax = Syntax::Typescript(TsConfig::default());
		let mut parser = Parser::new(syntax, input, None);

		// Parse the text.
		let module = parser
			.parse_module()
			.map_err(|error| error!("{}", error.into_kind().msg()))?;

		Ok(Output { module, source_map })
	}
}
