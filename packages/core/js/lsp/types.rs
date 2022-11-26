use crate::js;
use lsp_types as lsp;

impl From<js::compiler::types::Diagnostic> for lsp::Diagnostic {
	fn from(value: js::compiler::types::Diagnostic) -> Self {
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

impl From<js::compiler::types::Severity> for lsp::DiagnosticSeverity {
	fn from(value: js::compiler::types::Severity) -> Self {
		match value {
			js::compiler::types::Severity::Error => lsp::DiagnosticSeverity::ERROR,
			js::compiler::types::Severity::Warning => lsp::DiagnosticSeverity::WARNING,
			js::compiler::types::Severity::Information => lsp::DiagnosticSeverity::INFORMATION,
			js::compiler::types::Severity::Hint => lsp::DiagnosticSeverity::HINT,
		}
	}
}

impl From<js::compiler::types::Range> for lsp::Range {
	fn from(value: js::compiler::types::Range) -> Self {
		lsp::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Range> for js::compiler::types::Range {
	fn from(value: lsp::Range) -> Self {
		js::compiler::types::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Position> for js::compiler::types::Position {
	fn from(value: lsp::Position) -> Self {
		js::compiler::types::Position {
			line: value.line,
			character: value.character,
		}
	}
}

impl From<js::compiler::types::Position> for lsp::Position {
	fn from(value: js::compiler::types::Position) -> Self {
		lsp::Position {
			line: value.line,
			character: value.character,
		}
	}
}
