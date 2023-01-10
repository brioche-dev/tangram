use crate::compiler;
use lsp_types as lsp;

impl From<compiler::Diagnostic> for lsp::Diagnostic {
	fn from(value: compiler::Diagnostic) -> Self {
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

impl From<compiler::Severity> for lsp::DiagnosticSeverity {
	fn from(value: compiler::Severity) -> Self {
		match value {
			compiler::Severity::Error => lsp::DiagnosticSeverity::ERROR,
			compiler::Severity::Warning => lsp::DiagnosticSeverity::WARNING,
			compiler::Severity::Information => lsp::DiagnosticSeverity::INFORMATION,
			compiler::Severity::Hint => lsp::DiagnosticSeverity::HINT,
		}
	}
}

impl From<compiler::Range> for lsp::Range {
	fn from(value: compiler::Range) -> Self {
		lsp::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Range> for compiler::Range {
	fn from(value: lsp::Range) -> Self {
		compiler::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Position> for compiler::Position {
	fn from(value: lsp::Position) -> Self {
		compiler::Position {
			line: value.line,
			character: value.character,
		}
	}
}

impl From<compiler::Position> for lsp::Position {
	fn from(value: compiler::Position) -> Self {
		lsp::Position {
			line: value.line,
			character: value.character,
		}
	}
}
