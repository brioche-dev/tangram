use swc_core::common::Loc;

pub struct Error {
	message: String,
	line: usize,
	column: usize,
}

impl Error {
	pub fn new(message: impl std::fmt::Display, loc: &Loc) -> Self {
		let line = loc.line - 1;
		let column = loc.col_display;
		Self {
			message: message.to_string(),
			line,
			column,
		}
	}
}

impl std::fmt::Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let line = self.line + 1;
		let column = self.column + 1;
		let message = &self.message;
		write!(f, "{line}:{column} {message}").unwrap();
		Ok(())
	}
}
