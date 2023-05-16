use super::{
	mount::{self, Mount},
	server::Server,
	Process,
};
use crate::{
	error::{return_error, Result, WrapErr},
	instance::Instance,
	system::System,
	temp::Temp,
};
use indoc::writedoc;
use std::{
	collections::{BTreeMap, HashSet},
	ffi::{CStr, CString},
	fmt::Write,
	os::unix::prelude::OsStrExt,
	sync::Arc,
};

/// The home directory guest path.
const HOME_DIRECTORY_GUEST_PATH: &str = "/home/tangram";

/// The socket guest path.
const SOCKET_GUEST_PATH: &str = "/socket";

/// The working directory guest path.
const WORKING_DIRECTORY_GUEST_PATH: &str = "/home/tangram/work";

impl Process {
	#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
	pub async fn run_macos(
		tg: &Arc<Instance>,
		_system: System,
		executable: String,
		mut env: BTreeMap<String, String>,
		args: Vec<String>,
		mut mounts: HashSet<Mount, fnv::FnvBuildHasher>,
		network_enabled: bool,
	) -> Result<()> {
		// Create a temp for the root.
		let root_temp = Temp::new(tg);
		let root_host_path = root_temp.path().to_owned();
		std::fs::create_dir_all(&root_host_path)
			.wrap_err("Failed to create the root directory.")?;

		// Add the root directory to the mounts.
		mounts.insert(Mount {
			host_path: root_host_path.clone(),
			guest_path: root_host_path.clone(),
			mode: mount::Mode::ReadWrite,
			kind: mount::Kind::Directory,
		});

		// Add the aritfacts directory to the mounts.
		mounts.insert(Mount {
			host_path: tg.artifacts_path(),
			guest_path: tg.artifacts_path(),
			mode: mount::Mode::ReadOnly,
			kind: mount::Kind::Directory,
		});

		// Create the home directory.
		let home_directory_host_path =
			root_host_path.join(HOME_DIRECTORY_GUEST_PATH.strip_prefix('/').unwrap());
		tokio::fs::create_dir_all(&home_directory_host_path).await?;

		// Create the working directory.
		let working_directory_host_path =
			root_host_path.join(WORKING_DIRECTORY_GUEST_PATH.strip_prefix('/').unwrap());
		tokio::fs::create_dir_all(&working_directory_host_path).await?;

		// Set `$HOME`.
		env.insert(
			"HOME".to_owned(),
			home_directory_host_path.to_str().unwrap().to_owned(),
		);

		// Create the socket path and set `$TANGRAM_SOCKET`.
		let socket_host_path = root_host_path.join(SOCKET_GUEST_PATH.strip_prefix('/').unwrap());
		env.insert(
			"TANGRAM_SOCKET".to_owned(),
			socket_host_path.to_str().unwrap().to_owned(),
		);

		// Create the sandbox profile.
		let mut profile = String::new();

		// Write the default profile.
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
				(allow file-read* file-test-existence
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

		// Write the network profile.
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

		// Write the profile for the mounts.
		for mount in &mounts {
			writedoc!(
				profile,
				r#"
					(allow process-exec* (subpath {0}))
					(allow file-read* (path-ancestors {0}))
					(allow file-read* (subpath {0}))
				"#,
				escape(mount.host_path.as_os_str().as_bytes())
			)
			.unwrap();

			if mount.mode == mount::Mode::ReadWrite {
				writedoc!(
					profile,
					r#"
						(allow file-write* (subpath {0}))
					"#,
					escape(mount.host_path.as_os_str().as_bytes())
				)
				.unwrap();
			}
		}

		// Make the profile a C string.
		let profile = CString::new(profile).unwrap();

		// Start the server.
		let server = Server::new(
			Arc::downgrade(tg),
			tg.artifacts_path(),
			mounts.iter().cloned().collect(),
		);
		let server_task = tokio::spawn({
			let socket_path = socket_host_path.clone();
			async move {
				server.serve(&socket_path).await.unwrap();
			}
		});

		// Create the command.
		let mut command = tokio::process::Command::new(&executable);

		// Set the working directory.
		command.current_dir(&working_directory_host_path);

		// Set the envs.
		command.env_clear();
		command.envs(env);

		// Set the args.
		command.args(args);

		// Set up the sandbox.
		unsafe {
			command.pre_exec(move || {
				// Call `sandbox_init`.
				let error = std::ptr::null_mut::<*const libc::c_char>();
				let ret = sandbox_init(profile.as_ptr(), 0, error);

				// Handle an error from `sandbox_init`.
				if ret != 0 {
					let error = *error;
					let _message = CStr::from_ptr(error);
					sandbox_free_error(error);
					return Err(std::io::Error::from(std::io::ErrorKind::Other));
				}

				Ok(())
			})
		};

		// Spawn the child.
		let mut child = command.spawn().wrap_err("Failed to spawn the process.")?;

		// Wait for the child to exit.
		let status = child
			.wait()
			.await
			.wrap_err("Failed to wait for the process to exit.")?;

		// Stop the server.
		server_task.abort();
		server_task.await.ok();

		// Return an error if the process did not exit successfully.
		if !status.success() {
			return_error!("The process did not exit successfully.");
		}

		Ok(())
	}
}

extern "C" {
	fn sandbox_init(
		profile: *const libc::c_char,
		flags: u64,
		errorbuf: *mut *const libc::c_char,
	) -> libc::c_int;
	fn sandbox_free_error(errorbuf: *const libc::c_char) -> libc::c_void;
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
