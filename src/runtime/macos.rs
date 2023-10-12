use crate::{
	checkin, error, return_error, server, value::Value, Artifact, Client, Error, Result, Target,
	Template, WrapErr,
};
use futures::{stream::FuturesOrdered, TryStreamExt};
use indoc::writedoc;
use std::{
	ffi::{CStr, CString},
	fmt::Write,
	os::unix::prelude::OsStrExt,
	sync::Arc,
};

#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub async fn run_inner_macos(
	client: &dyn Client,
	target: Target,
	progress: Arc<server::build::State>,
) -> Result<Option<Value>> {
	// Get the server path.
	let server_path = client.path().unwrap().to_owned();

	// Get the artifacts path.
	let artifacts_path = server_path.join(".artifacts");

	// Create a tempdir for the root.
	let root_tempdir = tempfile::TempDir::new()?;
	let root_path = root_tempdir.path().to_owned();

	// Create a tempdir for the output.
	let output_tempdir = tempfile::TempDir::new()?;

	// Create the output parent directory.
	let output_parent_directory_path = output_tempdir.path().to_owned();
	tokio::fs::create_dir_all(&output_parent_directory_path)
		.await
		.wrap_err("Failed to create the output parent directory.")?;

	// Create the output path.
	let output_path = output_parent_directory_path.join("output");

	// Create the home directory.
	let home_directory_path = root_path.join("Users/tangram");
	tokio::fs::create_dir_all(&home_directory_path)
		.await
		.wrap_err("Failed to create the home directory.")?;

	// Create the working directory.
	let working_directory_path = root_path.join("Users/tangram/work");
	tokio::fs::create_dir_all(&working_directory_path)
		.await
		.wrap_err("Failed to create the working directory.")?;

	// Create a closure that renders a template with the artifacts and output guest paths.
	let render = {
		let client = client.clone();
		let artifacts_path = artifacts_path.clone();
		let output_path = output_path.clone();
		move |template: Template| async move {
			template
				.try_render(move |component| {
					let client = client.clone();
					let artifacts_path = artifacts_path.clone();
					let output_path = output_path.clone();
					async move { Ok("ello".to_owned()) }
				})
				.await
		}
	};
	// match component {
	// 	crate::template::Component::String(string) => Ok(string.clone()),
	// 	crate::template::Component::Artifact(artifact) => Ok(artifacts_path
	// 		.join(artifact.id(client).await?.to_string())
	// 		.into_os_string()
	// 		.into_string()
	// 		.unwrap()),
	// 	crate::template::Component::Placeholder(placeholder) => {
	// 		if placeholder.name == "output" {
	// 			Ok(output_path.as_os_str().to_str().unwrap().to_owned())
	// 		} else {
	// 			Err(error!(r#"Invalid placeholder "{}"."#, placeholder.name))
	// 		}
	// 	},
	// }

	// Render the executable.
	let executable = target.executable(client).await?;
	let executable = render(executable.clone()).await?;

	// Render the env.
	let env = target.env(client).await?;
	let env: std::collections::BTreeMap<String, String> = env
		.iter()
		.map(|(key, value)| async move {
			let key = key.clone();
			let value = value.try_unwrap_template_ref().unwrap();
			let value = render(value.clone()).await?;
			Ok::<_, Error>((key, value))
		})
		.collect::<FuturesOrdered<_>>()
		.try_collect()
		.await?;

	// Render the args.
	let args = target.args(client).await?;
	let args: Vec<String> = args
		.iter()
		.map(|value| {
			let value = value.try_unwrap_template_ref().unwrap();
			render(value.clone())
		})
		.collect::<FuturesOrdered<_>>()
		.try_collect()
		.await?;

	// Enable the network if a checksum was provided or if the unsafe flag was set.
	let network_enabled =
		target.checksum(client).await?.is_some() || target.unsafe_(client).await?;

	// Create the socket path.
	let socket_path = root_path.join("socket");

	// Set `$HOME`.
	env.insert(
		"HOME".to_owned(),
		home_directory_path.to_str().unwrap().to_owned(),
	);

	// Set `$TANGRAM_SOCKET`.
	env.insert(
		"TANGRAM_SOCKET".to_owned(),
		socket_path.to_str().unwrap().to_owned(),
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

			;; Allow executing /usr/bin/env and /bin/sh.
			(allow file-read* process-exec
				(literal "/usr/bin/env")
				(literal "/bin/sh")
				(literal "/bin/bash")
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

	// Allow read access to the artifacts directory.
	writedoc!(
		profile,
		r#"
			(allow process-exec* (subpath {0}))
			(allow file-read* (path-ancestors {0}))
			(allow file-read* (subpath {0}))
			(allow file-write* (subpath {0}))
		"#,
		escape(server_path.as_os_str().as_bytes())
	)
	.unwrap();

	// Allow write access to the home directory.
	writedoc!(
		profile,
		r#"
			(allow process-exec* (subpath {0}))
			(allow file-read* (path-ancestors {0}))
			(allow file-read* (subpath {0}))
			(allow file-write* (subpath {0}))
		"#,
		escape(home_directory_path.as_os_str().as_bytes())
	)
	.unwrap();

	// Allow write access to the output parent directory.
	writedoc!(
		profile,
		r#"
			(allow process-exec* (subpath {0}))
			(allow file-read* (path-ancestors {0}))
			(allow file-read* (subpath {0}))
			(allow file-write* (subpath {0}))
		"#,
		escape(output_parent_directory_path.as_os_str().as_bytes())
	)
	.unwrap();

	// Make the profile a C string.
	let profile = CString::new(profile).unwrap();

	// Create the command.
	let mut command = tokio::process::Command::new(&executable);

	// Set the working directory.
	command.current_dir(&working_directory_path);

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

	// Return an error if the process did not exit successfully.
	if !status.success() {
		return_error!("The process did not exit successfully.");
	}

	// Create the output.
	let value = if tokio::fs::try_exists(&output_path).await? {
		// Check in the output.
		let options = checkin::Options {
			artifacts_paths: vec![artifacts_path],
		};
		let artifact = Artifact::check_in_with_options(client, &output_path, &options)
			.await
			.wrap_err("Failed to check in the output.")?;

		// Verify the checksum if one was provided.
		if let Some(expected) = target.checksum(client).await?.clone() {
			let actual = artifact
				.checksum(client, expected.algorithm())
				.await
				.wrap_err("Failed to compute the checksum.")?;
			if expected != actual {
				return_error!(
					r#"The checksum did not match. Expected "{expected}" but got "{actual}"."#
				);
			}
		}

		artifact.into()
	} else {
		Value::Null(())
	};

	Ok(Some(value))
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
