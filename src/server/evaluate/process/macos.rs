use crate::server::Server;
use anyhow::{bail, Result};
use indoc::formatdoc;
use std::{
	collections::BTreeMap,
	ffi::{CStr, CString},
	os::unix::prelude::OsStrExt,
	path::Path,
	sync::Arc,
};

impl Server {
	pub(super) async fn run_macos_process(
		self: &Arc<Self>,
		envs: BTreeMap<String, String>,
		command: &Path,
		args: Vec<String>,
	) -> Result<()> {
		let server_path = self.path().to_owned();

		// Create the process.
		let mut process = tokio::process::Command::new(command);

		// Set the envs.
		process.env_clear();
		process.envs(envs);

		// Set the args.
		process.args(args);

		// Set up the sandbox.
		unsafe {
			process.pre_exec(move || {
				sandbox(&server_path)
					.map_err(|error| tokio::io::Error::new(tokio::io::ErrorKind::Other, error))
			})
		};

		// Spawn the process.
		let mut child = process.spawn()?;

		// Wait for the process to exit.
		child.wait().await?;

		Ok(())
	}
}

fn sandbox(server_path: &Path) -> Result<()> {
	let mut profile = String::new();

	// Add the default policy.
	profile.push_str(&formatdoc!(
		r#"
			(version 1)
			(deny default)

			;; Allow most system operations.
			(allow syscall*)
			(allow system-socket)
			(allow mach*)
			(allow ipc*)
			(allow sysctl*)

			;; Allow most process operations, EXCEPT for `process-exec`. `process-exec` will let you execute binaries without having been granted the corresponding `file-read*` permission.
			(allow process-fork process-info*)

			;; Allow TTY `ioctl()`s, so sandboxed interactive shells work smoothly.
			(allow file-ioctl (regex #"^/dev/ttys[0-9]+$"))

			;; Allow limited exploration of the root.
			(allow file-read-data (literal "/"))
			(allow file-read-metadata
				(literal "/Library")
				(literal "/System")
				(literal "/Users")
				(literal "/Volumes")
				(literal "/etc")
				(literal "/var"))

			;; Allow writing to common devices.
			(allow file-read* file-write-data file-ioctl
				(literal "/dev/null")
				(literal "/dev/zero")
				(literal "/dev/dtracehelper"))

			;; Allow reading some system devices and files.
			(allow file-read*
				(literal "/dev/autofs_nowait")
				(literal "/dev/random")
				(literal "/dev/urandom")
				(literal "/private/etc/protocols")
				(literal "/private/etc/services")
				(literal "/private/etc/localtime"))

			;; Support Rosetta.
			(allow file-read-metadata file-test-existence
				(literal "/Library/Apple/usr/libexec/oah/libRosettaRuntime"))
		"#
	));

	// Allow network access.
	profile.push_str(&formatdoc!(
		r#"
			;; Allow network access.
			(allow network*)

			;; Allow reading network preference files.
			(allow file-read*
				(literal "/Library/Preferences/com.apple.networkd.plist")
				(literal "/private/var/db/com.apple.networkextension.tracker-info")
				(literal "/private/var/db/nsurlstoraged/dafsaData.bin"))
			(allow user-preference-read (preference-domain "com.apple.CFNetwork"))

			;; (allow mach*) is included in the prelude, so all IPCs are allowed.

			;; (allow system-socket) is included in the prelude, so all sockets are allowed.
		"#
	));

	// Allow access to the tangram server path.
	profile.push_str(&formatdoc!(
		r#"
			(allow process-exec* file* (subpath {0}))
			(allow file-read-metadata (path-ancestors {0}))
		"#,
		escape(server_path.as_os_str().as_bytes())
	));

	// Call `sandbox_init`.
	let profile = CString::new(profile).unwrap();
	let mut error: *const c_char = std::ptr::null();
	let ret = unsafe { sandbox_init(profile.as_ptr(), 0, &mut error) };

	// Handle an error from `sandbox_init`.
	if ret != 0 {
		let message = unsafe { CStr::from_ptr(error) };
		let message = message.to_string_lossy();
		unsafe { sandbox_free_error(error) };
		bail!(message);
	}

	Ok(())
}

use libc::{c_char, c_int, c_void};
extern "C" {
	fn sandbox_init(profile: *const c_char, flags: u64, errorbuf: *mut *const c_char) -> c_int;
	fn sandbox_free_error(errorbuf: *const c_char) -> c_void;
}

/// Escape a string using the string literal syntax rules for `TinyScheme`. See <https://github.com/dchest/tinyscheme/blob/master/Manual.txt#L130>.
fn escape(bytes: impl AsRef<[u8]>) -> String {
	let bytes = bytes.as_ref();
	let mut output = String::new();
	output.push('"');
	for byte in bytes {
		let byte = *byte;
		match byte {
			b'"' => {
				output.push('\\');
				output.push('"');
			},
			b'\\' => {
				output.push('\\');
				output.push('\\');
			},
			b'\t' => {
				output.push('\\');
				output.push('t');
			},
			b'\n' => {
				output.push('\\');
				output.push('n');
			},
			b'\r' => {
				output.push('\\');
				output.push('r');
			},
			byte if char::from(byte).is_ascii_alphanumeric()
				|| char::from(byte).is_ascii_punctuation()
				|| byte == b' ' =>
			{
				output.push(byte.into());
			},
			byte => {
				output.push_str(&format!("\\x{:02X}", byte));
			},
		}
	}
	output.push('"');
	output
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_escape_string() {
		assert_eq!(escape(r#"quote ""#), r#""quote \"""#);
		assert_eq!(escape(r#"backslash \"#), r#""backslash \\""#);
		assert_eq!(escape("newline \n"), r#""newline \n""#);
		assert_eq!(escape("tab \t"), r#""tab \t""#);
		assert_eq!(escape("return \r"), r#""return \r""#);
		assert_eq!(escape("nul \0"), r#""nul \x00""#);
		assert_eq!(escape("many \r\t\n\\\r\n"), r#""many \r\t\n\\\r\n""#);
	}
}
