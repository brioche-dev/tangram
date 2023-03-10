//! The following command will log sandbox events:
//!
//! ```text
//! log stream --style compact --info --debug  --predicate '(((processID == 0) AND (senderImagePath CONTAINS "/Sandbox")) OR (subsystem == "com.apple.sandbox.reporting"))'
//! ```
//!
//! Helpful reference: <https://reverse.put.as/wp-content/uploads/2011/09/Apple-Sandbox-Guide-v1.0.pdf>.

use crate::{
	error::{bail, Context, Result},
	system::System,
	template::Path,
	Instance,
};
use indoc::writedoc;
use libc::{c_char, c_int, c_void};
use std::{
	collections::{BTreeMap, HashSet},
	ffi::{CStr, CString},
	fmt::Write,
	os::unix::prelude::OsStrExt,
};

impl Instance {
	pub async fn run_process_macos(
		&self,
		_system: System,
		env: BTreeMap<String, String>,
		command: String,
		args: Vec<String>,
		mut paths: HashSet<Path, fnv::FnvBuildHasher>,
		network_enabled: bool,
	) -> Result<()> {
		// Create a temp path for the root directory.
		let root_directory_path = self.temp_path();

		// Add the root directory to the paths.
		paths.insert(Path {
			host_path: root_directory_path.clone(),
			guest_path: root_directory_path.clone(),
			read: true,
			write: true,
			create: true,
		});

		// Add the home directory to the root directory.
		let home_directory_path = root_directory_path.join("Users").join("tangram");
		tokio::fs::create_dir_all(&home_directory_path).await?;

		// Add the working directory to the home directory.
		let working_directory_path = home_directory_path.join("work");
		tokio::fs::create_dir_all(&working_directory_path).await?;

		// Create the command.
		let mut command = tokio::process::Command::new(&command);

		// Set the working directory.
		command.current_dir(&working_directory_path);

		// Set the envs.
		command.env_clear();
		command.envs(env);
		command.env("HOME", &home_directory_path);

		// Set the args.
		command.args(args);

		// Set up the sandbox.
		unsafe {
			command.pre_exec(move || {
				pre_exec(&paths, network_enabled)
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

		// Remove the root directory.
		tokio::fs::remove_dir_all(root_directory_path).await?;

		// Error if the process did not exit successfully.
		if !status.success() {
			bail!("The process did not exit successfully.");
		}

		Ok(())
	}
}

#[allow(clippy::too_many_lines)]
fn pre_exec(paths: &HashSet<Path, fnv::FnvBuildHasher>, network_enabled: bool) -> Result<()> {
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
			)

			;; Allow writing to common devices.
			(allow file-read* file-write-data file-ioctl
				(literal "/dev/null")
				(literal "/dev/zero")
				(literal "/dev/dtracehelper")
			)

			;; Allow reading and writing temporary files.
			(allow file-write* file-read*
				(subpath "/tmp")
				(subpath "/private/tmp")
				(subpath "/private/var")
				(subpath "/var")
			)

			;; Allow reading some system devices and files.
			(allow file-read*
				(literal "/dev/autofs_nowait")
				(literal "/dev/random")
				(literal "/dev/urandom")
				(literal "/private/etc/protocols")
				(literal "/private/etc/services")
				(literal "/private/etc/localtime")
			)

			;; Allow /bin/sh and /usr/bin/env to execute.
			(allow process-exec
				(literal "/bin/bash")
				(literal "/bin/sh")
				(literal "/usr/bin/env")
			)

			;; Support Rosetta.
			(allow file-read-metadata file-test-existence
				(literal "/Library/Apple/usr/libexec/oah/libRosettaRuntime")
			)

			;; Allow accessing the dyld shared cache.
			(allow file-read* process-exec
				(literal "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld")
				(subpath "/System/Volumes/Preboot/Cryptexes/OS/System/Library/dyld")
			)

			;; Allow bash to create and use file descriptors for pipes.
			(allow file-read* file-write* file-ioctl process-exec
				(literal "/dev/fd")
				(subpath "/dev/fd")
			)
		"#
	).unwrap();

	// Add the network policy.
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
					(literal "/private/var/db/nsurlstoraged/dafsaData.bin")
				)
				(allow user-preference-read (preference-domain "com.apple.CFNetwork"))
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

	// Allow access to the paths in the path set.
	for entry in paths {
		if entry.read || entry.write || entry.create {
			writedoc!(
				profile,
				r#"
					(allow process-exec* (subpath {0}))
					(allow file-read* (path-ancestors {0}))
				"#,
				escape(entry.host_path.as_os_str().as_bytes())
			)
			.unwrap();
		}

		if entry.read {
			writedoc!(
				profile,
				r#"
					(allow file-read* (subpath {0}))
				"#,
				escape(entry.host_path.as_os_str().as_bytes())
			)
			.unwrap();
		}

		if entry.write {
			writedoc!(
				profile,
				r#"
					(allow file-write* (subpath {0}))
				"#,
				escape(entry.host_path.as_os_str().as_bytes())
			)
			.unwrap();
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
