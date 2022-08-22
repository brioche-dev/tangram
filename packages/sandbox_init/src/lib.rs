//! macOS process sandboxing using `sandbox_init`.
//!
//! # Examples
//!
//! To place the current process into a sandbox defined by one of the macOS standard profiles, call
//! [`sandbox_init_named`].
//!
//! The standard profiles supported in macOS are [`NO_INTERNET`], [`NO_NETWORK`], [`NO_WRITE`],
//! [`NO_WRITE_EXCEPT_TEMPORARY`], and [`PURE_COMPUTATION`].
//!
//! ```no_run
//! use sandbox_init::{sandbox_init_named, NO_INTERNET};
//!
//! sandbox_init_named(NO_INTERNET)
//!		.expect("Failed to sandbox this process");
//! ```
//!
//! For more advanced use, you can write a sandbox policy as an SBPL program. SBPL, which stands for
//! "Sandbox Policy Language," is a dialect of Scheme that macOS uses to implement App Store
//! sandboxing rules.
//!
//! To use a SBPL policy, call [`sandbox_init`] to sandbox the current process:
//!
//! ```no_run
//! use sandbox_init::sandbox_init;
//!
//! // Define the sandbox policy in SBPL
//! let policy = r#"
//!		(version 1)
//! 	(allow file-read*)
//! 	(deny file-write*)
//! "#;
//!
//! // Sandbox this process, following the logic in `policy`.
//! sandbox_init(policy)
//!		.expect("Failed to sandbox this process");
//! ```

use anyhow::{anyhow, Error, Result};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// Profile to disable TCP/IP networking.
pub static NO_INTERNET: NamedProfile =
	NamedProfile(unsafe { &sandbox_init_sys::kSBXProfileNoInternet as *const c_char });

/// Profile to disable all sockets-based networking.
pub static NO_NETWORK: NamedProfile =
	NamedProfile(unsafe { &sandbox_init_sys::kSBXProfileNoNetwork as *const c_char });

/// Profile to disable all filesystem writes.
pub static NO_WRITE: NamedProfile =
	NamedProfile(unsafe { &sandbox_init_sys::kSBXProfileNoWrite as *const c_char });

/// Profile to restrict all filesystem writes to the temporary folder `/var/tmp` and the folder
/// specified by the `confstr(3)` configuration variable `_CS_DARWIN_USER_TEMP_DIR`
pub static NO_WRITE_EXCEPT_TEMPORARY: NamedProfile =
	NamedProfile(unsafe { &sandbox_init_sys::kSBXProfileNoWriteExceptTemporary as *const c_char });

/// Profile to disable all operating system services.
pub static PURE_COMPUTATION: NamedProfile =
	NamedProfile(unsafe { &sandbox_init_sys::kSBXProfilePureComputation as *const c_char });

/// A named sandbox profile.
#[derive(Clone, Copy)]
pub struct NamedProfile(*const c_char);
unsafe impl Sync for NamedProfile {}

/// Place the current process into a sandbox defined by a named profile.
pub fn sandbox_init_named(profile: NamedProfile) -> Result<()> {
	// Out-parameter for an error message.
	let mut err_buf: *mut c_char = ptr::null_mut();

	unsafe {
		// Sandbox this process, using a named profile.
		let ret = sandbox_init_sys::sandbox_init(
			profile.0,
			sandbox_init_sys::SANDBOX_NAMED.into(), // convert to u64
			&mut err_buf,
		);

		if ret == 0 {
			Ok(())
		} else {
			Err(free_and_make_anyhow_error(err_buf))
		}
	}
}

/// Place the current process into a sandbox defined by a SBPL program.
pub fn sandbox_init(profile: &str) -> Result<()> {
	// Out-parameter for an error message.
	let mut err_buf: *mut c_char = ptr::null_mut();

	// Copy the profile into a CString so we can pass it to `sandbox_init`
	let profile = CString::new(profile)
		.map_err(|_| anyhow!("failed to convert sandbox profile to C string"))?;

	unsafe {
		let ret = sandbox_init_sys::sandbox_init(profile.as_ptr(), 0u64, &mut err_buf);

		if ret == 0 {
			Ok(())
		} else {
			Err(free_and_make_anyhow_error(err_buf))
		}
	}
}

/// Free an `err_buf` returned from `sandbox_init` by calling `sandbox_free_error`, returning the
/// text of the error as an [`anyhow::Error`]
fn free_and_make_anyhow_error(err_buf: *mut c_char) -> Error {
	// Convert the message buffer to a CStr
	let msg = unsafe { CStr::from_ptr(err_buf) };

	// Convert to a Rust string, using replacement characters if necessary.
	let string = msg.to_string_lossy();

	// Free the underlying error buffer
	unsafe { sandbox_init_sys::sandbox_free_error(err_buf) };

	anyhow!(string)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::env;
	use std::path::PathBuf;

	#[test]
	fn profile_name_is_valid() {
		unsafe {
			let s = std::ffi::CStr::from_ptr(NO_WRITE_EXCEPT_TEMPORARY.0);
			assert_eq!(s.to_str().unwrap(), "no-write-except-temporary")
		}
	}

	#[test]
	#[ignore = "this test sandboxes the test process, so it can only be run alone"]
	fn sandbox_this_test_no_write() {
		// Put the test process into a sandbox
		sandbox_init_named(NO_WRITE_EXCEPT_TEMPORARY).unwrap();

		// Assert that file creation fails in $HOME
		let path = PathBuf::from(env::var("HOME").unwrap()).join("file-should-fail");
		let fs_err = std::fs::File::create(path).unwrap_err();
		println!("Got error inside sandbox: {fs_err}");

		// Assert that file creation still works in /var/tmp
		let path = "/var/tmp/sandbox_init_crate_test_file__no_write";
		std::fs::File::create(path).unwrap();
		std::fs::remove_file(path).unwrap();
		println!("Created and removed: {path}")
	}

	#[test]
	#[ignore = "this test sandboxes the test process, so it can only be run alone"]
	fn sandbox_this_test_custom() {
		let policy = r#"
			(version 1)
			(allow file-read*)
			(deny file-write*)
		"#;
		sandbox_init(policy).expect("Failed to sandbox this process");

		// Try to read a file
		std::fs::read_to_string("/etc/shells").expect("failed to read file");

		// Try to write a file
		std::fs::File::create("/var/tmp/sandbox_init_crate_test_file__custom")
			.expect_err("file write mistakenly succeeded");
	}
}
