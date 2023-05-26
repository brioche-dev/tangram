use crate::{
	language::{Diagnostic, Severity},
	module::position::Position,
	module::range::Range,
};
use lsp_types as lsp;

impl From<Diagnostic> for lsp::Diagnostic {
	fn from(value: Diagnostic) -> Self {
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

impl From<Severity> for lsp::DiagnosticSeverity {
	fn from(value: Severity) -> Self {
		match value {
			Severity::Error => lsp::DiagnosticSeverity::ERROR,
			Severity::Warning => lsp::DiagnosticSeverity::WARNING,
			Severity::Information => lsp::DiagnosticSeverity::INFORMATION,
			Severity::Hint => lsp::DiagnosticSeverity::HINT,
		}
	}
}

impl From<Range> for lsp::Range {
	fn from(value: Range) -> Self {
		lsp::Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Range> for Range {
	fn from(value: lsp::Range) -> Self {
		Range {
			start: value.start.into(),
			end: value.end.into(),
		}
	}
}

impl From<lsp::Position> for Position {
	fn from(value: lsp::Position) -> Self {
		Position {
			line: value.line,
			character: value.character,
		}
	}
}

impl From<Position> for lsp::Position {
	fn from(value: Position) -> Self {
		lsp::Position {
			line: value.line,
			character: value.character,
		}
	}
}
