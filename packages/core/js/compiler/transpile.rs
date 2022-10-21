use super::Compiler;
use anyhow::{Context, Result};
use url::Url;

pub struct Output {
	pub transpiled_source: String,
	pub source_map: Option<String>,
}

impl Compiler {
	pub fn transpile(&self, url: &Url, source: &str) -> Result<Output> {
		// Parse the code.
		let parsed_source = deno_ast::parse_module(deno_ast::ParseParams {
			specifier: url.to_string(),
			text_info: deno_ast::SourceTextInfo::new(source.to_owned().into()),
			media_type: deno_ast::MediaType::TypeScript,
			capture_tokens: true,
			scope_analysis: true,
			maybe_syntax: None,
		})
		.with_context(|| format!(r#"Failed to parse the module with URL "{url}"."#))?;

		// Transpile the code.
		let transpiled_source = parsed_source
			.transpile(&deno_ast::EmitOptions {
				inline_source_map: false,
				..Default::default()
			})
			.with_context(|| format!(r#"Failed to transpile the module with URL "{url}"."#))?;

		Ok(Output {
			transpiled_source: transpiled_source.text,
			source_map: transpiled_source.source_map,
		})
	}
}
