//! The following command will display sandbox log events:
//!
//! ```
//! log stream --style compact --info --debug  --predicate '(((processID == 0) AND (senderImagePath CONTAINS "/Sandbox")) OR (subsystem == "com.apple.sandbox.reporting"))'
//! ```
//!

use super::{PathMode, ReferencedPathSet};
use crate::State;
use anyhow::{bail, Context, Result};
use indoc::writedoc;
use libc::{c_char, c_int, c_void};
use std::{
	collections::BTreeMap,
	ffi::{CStr, CString},
	fmt::Write,
	os::unix::prelude::OsStrExt,
	path::Path,
};

impl State {
	pub async fn run_process_macos(
		&self,
		env: BTreeMap<String, String>,
		command: String,
		args: Vec<String>,
		referenced_path_set: ReferencedPathSet,
		network_enabled: bool,
	) -> Result<()> {
		// Create a temp path for the working directory.
		let working_directory = self.create_temp_path();
		tokio::fs::create_dir_all(&working_directory).await?;

		// Create the command.
		let mut command = tokio::process::Command::new(&command);

		// Set the current dir.
		command.current_dir(&working_directory);

		// Set the envs.
		command.env_clear();
		command.envs(env);

		// Set the args.
		command.args(args);

		// Set up the sandbox.
		unsafe {
			command.pre_exec(move || {
				pre_exec(&referenced_path_set, &working_directory, network_enabled)
					.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
			})
		};

		// Spawn the child.
		let mut child = command.spawn().context("Failed to spawn the process.")?;

		// Wait for the child to exit.
		let status = child
			.wait()
			.await
			.context("Failed to wait for the process to exit.")?;

		// Error if the process does not exit successfully.
		if !status.success() {
			bail!("The process did not exit successfully.");
		}

		Ok(())
	}
}

#[allow(clippy::too_many_lines)]
fn pre_exec(
	referenced_path_set: &ReferencedPathSet,
	working_directory: &Path,
	network_enabled: bool,
) -> Result<()> {
	let mut profile = String::new();

	// Add the default policy.
	writedoc!(
		profile,
		r#"
			(version 1)

			;; Deny everything by default.
			(deny default)

			;; Allow most system operations.
			(allow syscall*)
			(allow system-socket)
			(allow mach*)
			(allow ipc*)
			(allow sysctl*)

			;; Allow most process operations, except for `process-exec`. `process-exec` will let you execute binaries without having been granted the corresponding `file-read*` permission.
			(allow process-fork process-info*)

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

			;; Allow reading and writing temporary files.
			(allow file-write* file-read*
				(subpath "/tmp")
				(subpath "/private/tmp")
				(subpath "/private/var/tmp"))

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
	).unwrap();

	// Allow network access if enabled.
	if network_enabled {
		writedoc!(
			profile,
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
		)
		.unwrap();
	} else {
		writedoc!(
			profile,
			r#"
				;; Disable global network access.
				(deny network*)

				;; Allow network access to localhost and Unix sockets
				(allow network* (remote ip "localhost:*"))
				(allow network* (remote unix-socket))
			"#
		)
		.unwrap();
	}

	// Allow access to the working directory.
	writedoc!(
		profile,
		r#"
			(allow process-exec* (subpath {0}))
			(allow file-read* (path-ancestors {0}))
			(allow file-read* (subpath {0}))
		"#,
		escape(working_directory.as_os_str().as_bytes())
	)
	.unwrap();

	// Allow access to all paths used in the build.
	for entry in referenced_path_set {
		match entry.mode {
			PathMode::Read => {
				writedoc!(
					profile,
					r#"
						(allow process-exec* (subpath {0}))
						(allow file-read* (path-ancestors {0}))
						(allow file-read* (subpath {0}))
					"#,
					escape(entry.path.as_os_str().as_bytes())
				)
				.unwrap();
			},
			PathMode::ReadWrite | PathMode::ReadWriteCreate => {
				writedoc!(
					profile,
					r#"
						(allow process-exec* (subpath {0}))
						(allow file-read* (path-ancestors {0}))
						(allow file* (subpath {0}))
					"#,
					escape(entry.path.as_os_str().as_bytes())
				)
				.unwrap();
			},
		}
	}

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
				write!(output, "\\x{byte:02X}").unwrap();
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
