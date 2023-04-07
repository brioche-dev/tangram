use crate::{
	error::{Result, WrapErr},
	module::Module,
};

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
	pub transpiled_text: String,
	pub source_map: String,
}

impl Module {
	#[allow(clippy::unused_async)]
	pub async fn transpile(text: String) -> Result<Output> {
		// Parse the code.
		let parsed_source = deno_ast::parse_module(deno_ast::ParseParams {
			specifier: "module".to_owned(),
			text_info: deno_ast::SourceTextInfo::new(text.into()),
			media_type: deno_ast::MediaType::TypeScript,
			capture_tokens: true,
			scope_analysis: true,
			maybe_syntax: None,
		})
		.ok()
		.wrap_err("Failed to parse the module.")?;

		// Transpile the code.
		let output = parsed_source
			.transpile(&deno_ast::EmitOptions {
				inline_source_map: false,
				..Default::default()
			})
			.ok()
			.wrap_err("Failed to transpile the module.")?;

		// Get the transpiled text and source map.
		let transpiled_text = output.text;
		let source_map = output.source_map.wrap_err("Expected a source map.")?;

		Ok(Output {
			transpiled_text,
			source_map,
		})
	}
}
