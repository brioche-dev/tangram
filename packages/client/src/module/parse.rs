use super::Module;
use crate::{error, Result};
use std::rc::Rc;
use swc_core as swc;

pub struct Output {
	pub program: swc::ecma::ast::Program,
	pub source_map: Rc<swc::common::SourceMap>,
}

impl Module {
	pub fn parse(text: String) -> Result<Output> {
		// Create the parser.
		let source_map = Rc::new(swc::common::SourceMap::default());
		let source_file = source_map.new_source_file(swc::common::FileName::Anon, text);
		let input = swc::ecma::parser::StringInput::from(&*source_file);
		let config = swc::ecma::parser::TsConfig::default();
		let syntax = swc::ecma::parser::Syntax::Typescript(config);
		let mut parser = swc::ecma::parser::Parser::new(syntax, input, None);

		// Parse the text.
		let program = parser
			.parse_program()
			.map_err(|error| error!("{}", error.into_kind().msg()))?;

		Ok(Output {
			program,
			source_map,
		})
	}
}
