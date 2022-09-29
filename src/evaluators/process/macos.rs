use super::Process;
use crate::{builder, system::System};
use anyhow::{bail, Context, Result};
use indoc::writedoc;
use std::{
	collections::BTreeMap,
	ffi::{CStr, CString},
	fmt::Write,
	os::unix::prelude::OsStrExt,
	path::{Path, PathBuf},
};

impl Process {
	pub(super) async fn run_macos_process(
		&self,
		builder: &builder::Shared,
		_system: System,
		envs: BTreeMap<String, String>,
		command: PathBuf,
		args: Vec<String>,
	) -> Result<()> {
		let path = builder.path().to_owned();

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
				pre_exec(&path)
					.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
			})
		};

		// Spawn the process.
		let mut child = process.spawn().context("Failed to spawn the process.")?;

		// Wait for the process to exit.
		let status = child
			.wait()
			.await
			.context("Failed to wait for the process to exit.")?;

		if !status.success() {
			bail!("The process did not exit successfully.");
		}

		Ok(())
	}
}

fn pre_exec(path: &Path) -> Result<()> {
	let mut profile = String::new();

	// Add the default policy.
	writedoc!(
		profile,
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
	).unwrap();

	// Allow network access.
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

	// Allow access to the builder path.
	writedoc!(
		profile,
		r#"
			(allow process-exec* file* (subpath {0}))
			(allow file-read-metadata (path-ancestors {0}))
		"#,
		escape(path.as_os_str().as_bytes())
	)
	.unwrap();

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
