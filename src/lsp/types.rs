use crate::compiler;
use lsp_types as lsp;

impl From<compiler::types::Diagnostic> for lsp::Diagnostic {
	fn from(value: compiler::types::Diagnostic) -> Self {
		let range = value
			.location
			.map(|location| location.range.into())
			.unwrap_or_default();
		let severity = Some(value.severity.into());
		let source = Some("tangram".to_owned());
		let message = value.message;
		lsp::Diagnostic {
			range,
			severity,
			source,
			message,
			..Default::default()
		}
	}
}

impl From<compiler::types::Severity> for lsp::DiagnosticSeverity {
	fn from(value: compiler::types::Severity) -> Self {
		match value {
			compiler::types::Severity::Error => lsp::DiagnosticSeverity::ERROR,
			compiler::types::Severity::Warning => lsp::DiagnosticSeverity::WARNING,
			compiler::types::Severity::Information => lsp::DiagnosticSeverity::INFORMATION,
			compiler::types::Severity::Hint => lsp::DiagnosticSeverity::HINT,
		}
	}
}

impl From<compiler::types::Range> for lsp::Range {
	fn from(value: compiler::types::Range) -> Self {
		lsp::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Range> for compiler::types::Range {
	fn from(value: lsp::Range) -> Self {
		compiler::types::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Position> for compiler::types::Position {
	fn from(value: lsp::Position) -> Self {
		compiler::types::Position {
			line: value.line,
			character: value.character,
		}
	}
}

impl From<compiler::types::Position> for lsp::Position {
	fn from(value: compiler::types::Position) -> Self {
		lsp::Position {
			line: value.line,
			character: value.character,
		}
	}
}
