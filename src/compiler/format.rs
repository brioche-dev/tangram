use super::{analyze, ModuleIdentifier, TextEdit};
use crate::Cli;
use anyhow::Result;
use rome_rowan::AstNode;

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatRequest {
	pub module_identifier: ModuleIdentifier,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatResponse {
	pub edits: Option<Vec<TextEdit>>,
}

impl Cli {
	#[allow(clippy::unused_async)]
	pub async fn format(
		&self,
		module_identifier: ModuleIdentifier,
	) -> Result<Option<Vec<TextEdit>>> {
		// Load the source code of the module.
		let code = self.load(&module_identifier).await?;

		// Parse the module as TypeScript and get the abstract syntax tree.
		let source_type = rome_js_syntax::SourceType::ts();
		let source_ast_root = rome_js_parser::parse(&code, source_type).tree();
		let source_ast_root_node = source_ast_root.syntax();

		// Format the module using tabs for indentation.
		let format_options = rome_js_formatter::context::JsFormatOptions::new(source_type)
			.with_indent_style(rome_formatter::IndentStyle::Tab);
		let formatted_code =
			rome_js_formatter::format_sub_tree(format_options, source_ast_root_node)?;

		// Convert the formatted code back to a string.
		let formatted_code = formatted_code.as_code();

		// Re-parse the formatted string back to an AST.
		let mut formatted_ast_root = rome_js_parser::parse(formatted_code, source_type).tree();

		// Automatically apply fixes for a subset of linting rules.
		let rule_filter = rome_analyze::AnalysisFilter::from_enabled_rules(Some(&[
			rome_analyze::RuleFilter::Rule("correctness", "organizeImports"),
			rome_analyze::RuleFilter::Rule("tangramFormat", "templateIndent"),
		]));
		analyze::fix_all(&mut formatted_ast_root, rule_filter)?;

		// Get the start and end of the file. Replacing this range should replace the whole file.
		let range = super::Range::from_byte_range_in_string(&code, 0..code.len());

		// Convert the formatted and linted AST back to a string.
		let updated_code = formatted_ast_root.syntax().to_string();

		// Return an edit that replaces the file with the formatted code.
		Ok(Some(vec![TextEdit {
			range,
			new_text: updated_code,
		}]))
	}
}
