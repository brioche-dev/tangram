use anyhow::{ensure, Result};
use camino::Utf8Path;

/// The policy prelude. Sets up access to needed system files and resources.
const POLICY_PRELUDE: &str = include_str!("prelude.sb");

/// Rules to enable access to the network.
const ENABLE_NETWORK_INCLUDE: &str = include_str!("enable_network.inc.sb");

/// Builder for SBPL programs, to be used with `sandbox_init`
#[derive(derive_more::Display)]
pub struct PolicyBuilder {
	contents: String,
}

impl PolicyBuilder {
	/// Create a new policy which enforces sandbox limits.
	pub fn new() -> PolicyBuilder {
		let mut contents = String::from(
			r#"
            (version 1)
            (deny default)
            "#,
		);
		contents.push_str(POLICY_PRELUDE);
		PolicyBuilder { contents }
	}

	/// Create a new policy which allows and logs policy violations.
	///
	/// To see the violation logs, run the following, where `$PID` is the pid of the process being
	/// sandboxed:
	///
	/// ```sh
	/// log stream --predicate \
	///     "process == 'kernel' and sender == 'Sandbox' and composedMessage contains '$PID'"
	/// ```
	#[allow(dead_code)]
	pub fn new_advisory() -> PolicyBuilder {
		let mut contents = String::from(
			r#"
            (version 1)
            (allow (with report) default)
            "#,
		);
		contents.push_str(POLICY_PRELUDE);
		PolicyBuilder { contents }
	}

	/// Allow access to the network.
	pub fn allow_network(&mut self) -> &mut Self {
		self.contents.push_str(ENABLE_NETWORK_INCLUDE);
		self
	}

	/// Allow reads to a file or directory, referenced by exact path.
	///
	/// `path` must be an absolute path.
	pub fn allow_read(&mut self, path: &Utf8Path) -> Result<&mut Self> {
		ensure!(
			path.is_absolute(),
			"read-only sandbox paths must be configured using an absolute path"
		);
		let expr = format!(
			r#"
			(allow process-exec* file-read* (literal {0}))
			"#,
			Self::string_literal(path)
		);
		self.contents.push_str(&expr);
		Ok(self)
	}

	/// Allow file reads to a path, and all files contained within it.
	///
	/// `path` must be an absolute path.
	pub fn allow_read_subpath(&mut self, path: &Utf8Path) -> Result<&mut Self> {
		ensure!(
			path.is_absolute(),
			"read-only sandbox subpaths must be configured using an absolute path"
		);
		// `(allow file-read* ...)` permits any read for files within the subpath
		// `(allow file-read-metadata ...)` is required for `pwd` and the like to function
		let expr = format!(
			r#"
			(allow process-exec* file-read* (subpath {0}))
			(allow file-read-metadata (path-ancestors {0}))
			"#,
			Self::string_literal(path)
		);
		self.contents.push_str(&expr);
		Ok(self)
	}

	/// Allow all file operations to a path, and all files contained within it.
	///
	/// `path` must be an absolute path.
	pub fn allow_write_subpath(&mut self, path: &Utf8Path) -> Result<&mut Self> {
		ensure!(
			path.is_absolute(),
			"read-write sandbox subpaths must be configured using an absolute path"
		);
		// `(allow file* ...)` permits any access to files within the subpath
		// `(allow file-read-metadata ...)` is required for `pwd` and the like to function
		let expr = format!(
			r#"
			(allow file* (subpath {0}))
			(allow file-read-metadata (path-ancestors {0}))
			"#,
			Self::string_literal(path)
		);
		self.contents.push_str(&expr);
		Ok(self)
	}

	/// Escape a string, using the string literal syntax rules for TinyScheme.
	///
	/// See: <https://github.com/dchest/tinyscheme/blob/master/Manual.txt#L130>
	fn string_literal(source: impl std::fmt::Display) -> String {
		let contents: String = source
			.to_string()
			.chars()
			.flat_map(|ch| {
				match ch {
					// Escape `"` with `\"`
					'"' => vec!['\\', '\"'],

					// Escape `\` with `\\`
					'\\' => vec!['\\', '\\'],

					// Tabs, line returns, newlines
					'\t' => vec!['\\', 't'],
					'\n' => vec!['\\', 'n'],
					'\r' => vec!['\\', 'r'],

					// ASCII alphanumeric chars go through untouched
					ch if ch.is_ascii_alphanumeric() || ch.is_ascii_punctuation() || ch == ' ' => {
						vec![ch]
					},

					// Everything else gets hex-encoded.
					ch => format!("\\x{:02X}", ch as u8).chars().collect(),
				}
			})
			.collect();

		format!("\"{contents}\"")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn string_literal_escape() {
		// (raw) -> (escaped)
		let pairs = [
			(r#"quote --"--"#, r#""quote --\"--""#),
			(r#"backslash --\--"#, r#""backslash --\\--""#),
			("newline \n", r#""newline \n""#),
			("tab \t", r#""tab \t""#),
			("return \r", r#""return \r""#),
			("nul \0", r#""nul \x00""#),
			("many \r\t\n\\\r\n", r#""many \r\t\n\\\r\n""#),
		];

		for (raw, escaped) in pairs {
			let got = PolicyBuilder::string_literal(raw);

			println!();
			println!("escaped  :  {escaped}");
			println!("got      : {got}");
			println!("raw     ?:  {raw:?}");
			println!("escaped ?:  {escaped:?}");
			println!("got     ?: {got:?}");

			assert_eq!(got, escaped);
		}
	}
}
