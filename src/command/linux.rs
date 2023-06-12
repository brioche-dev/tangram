use super::Command;
use crate::{
	artifact::{self, Artifact},
	command,
	error::{return_error, Error, Result, WrapErr},
	instance::Instance,
	operation,
	system::System,
	temp::Temp,
	value::Value,
};
use indoc::formatdoc;
use itertools::Itertools;
use std::{
	ffi::CString,
	os::{fd::AsRawFd, unix::ffi::OsStrExt},
	path::{Path, PathBuf},
	sync::Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The home directory guest path.
const HOME_DIRECTORY_GUEST_PATH: &str = "/home/tangram";

/// The socket guest path.
const SOCKET_GUEST_PATH: &str = "/socket";

/// The tangram directory guest path.
const TANGRAM_DIRECTORY_GUEST_PATH: &str = "/.tangram";

/// The UID for the tangram user.
const TANGRAM_UID: libc::uid_t = 1000;

/// The GID for the tangram user.
const TANGRAM_GID: libc::gid_t = 1000;

/// The working directory guest path.
const WORKING_DIRECTORY_GUEST_PATH: &str = "/home/tangram/work";

impl Command {
	#[allow(clippy::too_many_lines, clippy::similar_names)]
	pub async fn run_inner_linux(&self, tg: &Arc<Instance>) -> Result<Value> {
		// Check out the references.
		self.check_out_references(tg)
			.await
			.wrap_err("Failed to check out the references.")?;

		// Create a temp for the root.
		let root_temp = Temp::new(tg);
		let root_host_path = root_temp.path().to_owned();
		tokio::fs::create_dir_all(&root_host_path)
			.await
			.wrap_err("Failed to create the root directory.")?;

		// Create a temp for the output.
		let output_temp = Temp::new(tg);

		// Create the host and guest paths for the output parent directory.
		let output_parent_directory_host_path = output_temp.path().to_owned();
		let output_parent_directory_guest_path =
			PathBuf::from(format!("/.tangram/temps/{}", output_temp.id()));
		tokio::fs::create_dir_all(&output_parent_directory_host_path)
			.await
			.wrap_err("Failed to create the output parent directory.")?;

		// Create the host and guest paths for the output.
		let output_host_path = output_parent_directory_host_path.join("output");
		let output_guest_path = output_parent_directory_guest_path.join("output");

		// Create the host and guest paths for the tangram directory.
		let tangram_directory_host_path = tg.path().to_owned();
		let tangram_directory_guest_path = PathBuf::from(TANGRAM_DIRECTORY_GUEST_PATH);

		// Create the host and guest paths for the artifacts directory.
		let _artifacts_directory_guest_path = tg.artifacts_path().to_owned();
		let artifacts_directory_guest_path =
			Path::new(TANGRAM_DIRECTORY_GUEST_PATH).join("artifacts");

		// Create the host and guest paths for the home directory.
		let home_directory_host_path =
			root_host_path.join(HOME_DIRECTORY_GUEST_PATH.strip_prefix('/').unwrap());
		let _home_directory_guest_path = PathBuf::from(HOME_DIRECTORY_GUEST_PATH);
		tokio::fs::create_dir_all(&home_directory_host_path)
			.await
			.wrap_err(r#"Failed to create the home directory."#)?;

		// Create the host and guest paths for the working directory.
		let working_directory_host_path =
			root_host_path.join(WORKING_DIRECTORY_GUEST_PATH.strip_prefix('/').unwrap());
		tokio::fs::create_dir_all(&working_directory_host_path)
			.await
			.wrap_err(r#"Failed to create the working directory."#)?;

		// Render the executable, env, and args.
		let (executable, mut env, args) =
			self.render(&artifacts_directory_guest_path, &output_guest_path)?;

		// Enable unsafe options if a checksum was provided or if the unsafe flag was set.
		let enable_unsafe = self.checksum.is_some() || self.unsafe_;

		// Verify the safety constraints.
		if !enable_unsafe {
			if self.network {
				return_error!("Network access is not allowed in safe processes.");
			}
			if !self.host_paths.is_empty() {
				return_error!("Host paths are not allowed in safe processes.");
			}
		}

		// Handle the network flag.
		let network_enabled = self.network;

		// Get the `env` host path.
		let env_file_name = match self.system {
			System::Amd64Linux => "env_amd64_linux",
			System::Arm64Linux => "env_arm64_linux",
			_ => unreachable!(),
		};
		let env_host_path = tg.assets_path().join(env_file_name);

		// Get the `sh` host path.
		let sh_file_name = match self.system {
			System::Amd64Linux => "sh_amd64_linux",
			System::Arm64Linux => "sh_arm64_linux",
			_ => unreachable!(),
		};
		let sh_host_path = tg.assets_path().join(sh_file_name);

		// Set `$HOME`.
		env.insert("HOME".to_owned(), HOME_DIRECTORY_GUEST_PATH.to_owned());

		// Set `$TANGRAM_PATH`.
		env.insert(
			"TANGRAM_PATH".to_owned(),
			TANGRAM_DIRECTORY_GUEST_PATH.to_owned(),
		);

		// Set `$TG_PLACEHOLDER_OUTPUT`.
		env.insert(
			"TANGRAM_PLACEHOLDER_OUTPUT".to_owned(),
			output_guest_path.to_str().unwrap().to_owned(),
		);

		// Set `$TANGRAM_SOCKET`.
		env.insert(String::from("TANGRAM_SOCKET"), SOCKET_GUEST_PATH.to_owned());

		// Create /etc.
		tokio::fs::create_dir_all(root_host_path.join("etc"))
			.await
			.wrap_err("Failed to create /etc.")?;

		// Create /etc/passwd.
		tokio::fs::write(
			root_host_path.join("etc/passwd"),
			formatdoc!(
				r#"
					root:!:0:0:root:/nonexistent:/bin/false
					tangram:!:{TANGRAM_UID}:{TANGRAM_GID}:tangram:{HOME_DIRECTORY_GUEST_PATH}:/bin/false
					nobody:!:65534:65534:nobody:/nonexistent:/bin/false
				"#
			),
		)
		.await
		.wrap_err("Failed to create /etc/passwd.")?;

		// Create /etc/group.
		tokio::fs::write(
			root_host_path.join("etc/group"),
			formatdoc!(
				r#"
					tangram:x:{TANGRAM_GID}:tangram
				"#
			),
		)
		.await
		.wrap_err("Failed to create /etc/group.")?;

		// Create /etc/nsswitch.conf.
		tokio::fs::write(
			root_host_path.join("etc/nsswitch.conf"),
			formatdoc!(
				r#"
					passwd: files compat
					shadow: files compat
					hosts: files dns compat
				"#
			),
		)
		.await
		.wrap_err("Failed to create /etc/nsswitch.conf.")?;

		// If network access is enabled, then copy /etc/resolv.conf from the host.
		if network_enabled {
			tokio::fs::copy("/etc/resolv.conf", root_host_path.join("etc/resolv.conf"))
				.await
				.wrap_err("Failed to copy /etc/resolv.conf.")?;
		}

		// Create the socket.
		let (mut host_socket, guest_socket) = tokio::net::UnixStream::pair()
			.map_err(Error::other)
			.wrap_err("Failed to create the socket pair.")?;
		let guest_socket = guest_socket.into_std()?;
		guest_socket.set_nonblocking(false)?;

		// Create the mounts.
		let mut mounts = Vec::new();

		// Add /dev to the mounts.
		let dev_host_path = Path::new("/dev");
		let dev_guest_path = Path::new("/dev");
		let dev_source_path = dev_host_path;
		let dev_target_path = root_host_path.join(dev_guest_path.strip_prefix("/").unwrap());
		tokio::fs::create_dir_all(&dev_target_path)
			.await
			.wrap_err(r#"Failed to create the mountpoint for "/dev"."#)?;
		let dev_source_path = CString::new(dev_source_path.as_os_str().as_bytes()).unwrap();
		let dev_target_path = CString::new(dev_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: dev_source_path,
			target: dev_target_path,
			fstype: None,
			flags: libc::MS_BIND | libc::MS_REC,
			data: None,
			readonly: false,
		});

		// Add /proc to the mounts.
		let proc_host_path = Path::new("/proc");
		let proc_guest_path = Path::new("/proc");
		let proc_source_path = proc_host_path;
		let proc_target_path = root_host_path.join(proc_guest_path.strip_prefix("/").unwrap());
		tokio::fs::create_dir_all(&proc_target_path)
			.await
			.wrap_err(r#"Failed to create the mount point for "/proc"."#)?;
		let proc_source_path = CString::new(proc_source_path.as_os_str().as_bytes()).unwrap();
		let proc_target_path = CString::new(proc_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: proc_source_path,
			target: proc_target_path,
			fstype: Some(CString::new("proc").unwrap()),
			flags: 0,
			data: None,
			readonly: false,
		});

		// Add /tmp to the mounts.
		let tmp_host_path = Path::new("/tmp");
		let tmp_guest_path = Path::new("/tmp");
		let tmp_source_path = tmp_host_path;
		let tmp_target_path = root_host_path.join(tmp_guest_path.strip_prefix("/").unwrap());
		tokio::fs::create_dir_all(&tmp_target_path)
			.await
			.wrap_err(r#"Failed to create the mount point for "/tmp"."#)?;
		let tmp_source_path = CString::new(tmp_source_path.as_os_str().as_bytes()).unwrap();
		let tmp_target_path = CString::new(tmp_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: tmp_source_path,
			target: tmp_target_path,
			fstype: Some(CString::new("tmpfs").unwrap()),
			flags: 0,
			data: None,
			readonly: false,
		});

		// Add /usr/bin/env to the mounts.
		let usr_bin_env_host_path = env_host_path;
		let usr_bin_env_guest_path = Path::new("/usr/bin/env");
		let usr_bin_env_source_path = usr_bin_env_host_path;
		let usr_bin_env_target_path =
			root_host_path.join(usr_bin_env_guest_path.strip_prefix("/").unwrap());
		tokio::fs::create_dir_all(usr_bin_env_target_path.parent().unwrap())
			.await
			.wrap_err(r#"Failed to create the parent for the mount point for "/usr/bin/env"."#)?;
		tokio::fs::write(&usr_bin_env_target_path, "")
			.await
			.wrap_err(r#"Failed to create the mount point for "/usr/bin/env"."#)?;
		let usr_bin_env_source_path =
			CString::new(usr_bin_env_source_path.as_os_str().as_bytes()).unwrap();
		let usr_bin_env_target_path =
			CString::new(usr_bin_env_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: usr_bin_env_source_path,
			target: usr_bin_env_target_path,
			fstype: None,
			flags: libc::MS_BIND | libc::MS_REC,
			data: None,
			readonly: true,
		});

		// Add /bin/sh to the mounts.
		let bin_sh_host_path = sh_host_path;
		let bin_sh_guest_path = Path::new("/bin/sh");
		let bin_sh_source_path = bin_sh_host_path;
		let bin_sh_target_path = root_host_path.join(bin_sh_guest_path.strip_prefix("/").unwrap());
		tokio::fs::create_dir_all(bin_sh_target_path.parent().unwrap())
			.await
			.wrap_err(r#"Failed to create the parent for the mount point for "/bin/sh"."#)?;
		tokio::fs::write(&bin_sh_target_path, "")
			.await
			.wrap_err(r#"Failed to create the mount point for "/bin/sh"."#)?;
		let bin_sh_source_path = CString::new(bin_sh_source_path.as_os_str().as_bytes()).unwrap();
		let bin_sh_target_path = CString::new(bin_sh_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: bin_sh_source_path,
			target: bin_sh_target_path,
			fstype: None,
			flags: libc::MS_BIND | libc::MS_REC,
			data: None,
			readonly: true,
		});

		// Add the host paths to the mounts.
		for host_path in &self.host_paths {
			let host_path = Path::new(host_path);
			let guest_path = Path::new(host_path);
			let source_path = host_path;
			let target_path = root_host_path.join(guest_path.strip_prefix("/").unwrap());
			tokio::fs::create_dir_all(&target_path)
				.await
				.wrap_err(format!(
					r#"Failed to create the mount point for "{}"."#,
					host_path.display()
				))?;
			let source_path = CString::new(source_path.as_os_str().as_bytes()).unwrap();
			let target_path = CString::new(target_path.as_os_str().as_bytes()).unwrap();
			mounts.push(Mount {
				source: source_path,
				target: target_path,
				fstype: None,
				flags: libc::MS_BIND | libc::MS_REC,
				data: None,
				readonly: true,
			});
		}

		// Add the tangram directory to the mounts.
		let tangram_directory_source_path = tangram_directory_host_path;
		let tangram_directory_target_path =
			root_host_path.join(tangram_directory_guest_path.strip_prefix("/").unwrap());
		tokio::fs::create_dir_all(&tangram_directory_target_path)
			.await
			.wrap_err(r#"Failed to create the mount point for the tangram directory."#)?;
		let tangram_directory_source_path =
			CString::new(tangram_directory_source_path.as_os_str().as_bytes()).unwrap();
		let tangram_directory_target_path =
			CString::new(tangram_directory_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: tangram_directory_source_path,
			target: tangram_directory_target_path,
			fstype: None,
			flags: libc::MS_BIND | libc::MS_REC,
			data: None,
			// TODO: Only the database and artifacts created by the guest process should be write-able.
			readonly: false,
		});

		// Add the home directory to the mounts.
		let home_directory_source_path = home_directory_host_path.clone();
		let home_directory_target_path = home_directory_host_path.clone();
		let home_directory_source_path =
			CString::new(home_directory_source_path.as_os_str().as_bytes()).unwrap();
		let home_directory_target_path =
			CString::new(home_directory_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: home_directory_source_path,
			target: home_directory_target_path,
			fstype: None,
			flags: libc::MS_BIND | libc::MS_REC,
			data: None,
			readonly: false,
		});

		// Add the output parent directory to the mounts.
		let output_parent_directory_source_path = output_parent_directory_host_path.clone();
		let output_parent_directory_target_path = root_host_path.join(
			output_parent_directory_guest_path
				.strip_prefix("/")
				.unwrap(),
		);
		tokio::fs::create_dir_all(&output_parent_directory_target_path)
			.await
			.wrap_err(r#"Failed to create the mount point for the output parent directory."#)?;
		let output_parent_directory_source_path =
			CString::new(output_parent_directory_source_path.as_os_str().as_bytes()).unwrap();
		let output_parent_directory_target_path =
			CString::new(output_parent_directory_target_path.as_os_str().as_bytes()).unwrap();
		mounts.push(Mount {
			source: output_parent_directory_source_path,
			target: output_parent_directory_target_path,
			fstype: None,
			flags: libc::MS_BIND | libc::MS_REC,
			data: None,
			readonly: false,
		});

		// Create the executable.
		let executable = CString::new(executable)
			.map_err(Error::other)
			.wrap_err("The executable is not a valid C string.")?;

		// Create `envp`.
		let env = env
			.into_iter()
			.map(|(key, value)| format!("{key}={value}"))
			.map(|entry| CString::new(entry).unwrap())
			.collect_vec();
		let mut envp = Vec::with_capacity(env.len() + 1);
		for entry in env {
			envp.push(entry);
		}
		let envp = CStringVec::new(envp);

		// Create `argv`.
		let args: Vec<_> = args
			.into_iter()
			.map(|arg| CString::new(arg).map_err(Error::other))
			.try_collect()?;
		let mut argv = Vec::with_capacity(1 + args.len() + 1);
		argv.push(executable.clone());
		for arg in args {
			argv.push(arg);
		}
		let argv = CStringVec::new(argv);

		// Get the root host path as a C string.
		let root_host_path = CString::new(root_host_path.as_os_str().as_bytes())
			.map_err(Error::other)
			.wrap_err("The root host path is not a valid C string.")?;

		// Get the working directory guest path as a C string.
		let working_directory_guest_path = CString::new(WORKING_DIRECTORY_GUEST_PATH)
			.map_err(Error::other)
			.wrap_err("The working directory is not a valid C string.")?;

		// Create the context.
		let context = Context {
			argv,
			envp,
			executable,
			guest_socket,
			mounts,
			network_enabled,
			root_host_path,
			working_directory_guest_path,
		};

		// Spawn the root process.
		let root_process_pid = clone3(libc::CLONE_NEWUSER as _);
		if root_process_pid == -1 {
			return Err(Error::last_os_error().wrap("Failed to spawn the root process."));
		}

		// We are the child process. Execute the root function.
		if root_process_pid == 0 {
			root(context);
		}

		// Receive the guest process's PID from the socket.
		let guest_process_pid: libc::pid_t = host_socket
			.read_i32_le()
			.await
			.wrap_err("Failed to receive the PID of the guest process from the socket.")?;

		// Write the guest process's UID map.
		let uid = unsafe { libc::getuid() };
		tokio::fs::write(
			format!("/proc/{guest_process_pid}/uid_map"),
			format!("{TANGRAM_UID} {uid} 1\n"),
		)
		.await
		.wrap_err("Failed to set the UID map.")?;

		// Deny setgroups to the process.
		tokio::fs::write(format!("/proc/{guest_process_pid}/setgroups"), "deny")
			.await
			.wrap_err("Failed to disable setgroups.")?;

		// Write the guest process's GID map.
		let gid = unsafe { libc::getgid() };
		tokio::fs::write(
			format!("/proc/{guest_process_pid}/gid_map"),
			format!("{TANGRAM_GID} {gid} 1\n"),
		)
		.await
		.wrap_err("Failed to set the GID map.")?;

		// Notify the guest process that it can continue.
		host_socket
			.write_u8(1)
			.await
			.wrap_err("Failed to notify the guest process that it can continue.")?;

		// Receive the exit status of the guest process from the root process.
		let kind = host_socket
			.read_u8()
			.await
			.wrap_err("Failed to receive the exit status kind from the root process.")?;
		let value = host_socket
			.read_i32_le()
			.await
			.wrap_err("Failed to receive the exit status value from the root process.")?;
		let exit_status = match kind {
			0 => ExitStatus::Code(value),
			1 => ExitStatus::Signal(value),
			_ => unreachable!(),
		};

		// Wait for the root process.
		tokio::task::spawn_blocking(move || {
			let mut status: libc::c_int = 0;
			let ret = unsafe { libc::waitpid(root_process_pid, &mut status, libc::__WALL) };
			if ret == -1 {
				return Err(Error::last_os_error().wrap("Failed to wait for the root process."));
			}
			let root_process_exit_status = if libc::WIFEXITED(status) {
				let status = libc::WEXITSTATUS(status);
				ExitStatus::Code(status)
			} else if libc::WIFSIGNALED(status) {
				let signal = libc::WTERMSIG(status);
				ExitStatus::Signal(signal)
			} else {
				unreachable!();
			};
			if root_process_exit_status != ExitStatus::Code(0) {
				return_error!("The root process did not exit successfully.");
			}
			Ok(())
		})
		.await
		.map_err(Error::other)
		.wrap_err("Failed to join the process task.")?
		.wrap_err("Failed to run the process.")?;

		// Handle the guest process's exit status.
		match exit_status {
			ExitStatus::Code(0) => {},
			ExitStatus::Code(code) => {
				return Err(Error::Operation(operation::Error::Command(
					command::Error::Code(code),
				)))
			},
			ExitStatus::Signal(signal) => {
				return Err(Error::Operation(operation::Error::Command(
					command::Error::Signal(signal),
				)))
			},
		};

		tracing::debug!(?output_host_path, "Checking in the process output.");

		// Create the output.
		let value = if tokio::fs::try_exists(&output_host_path).await? {
			// Check in the output.
			let options = artifact::checkin::Options {
				artifacts_paths: vec![artifacts_directory_guest_path],
			};
			let artifact = Artifact::check_in_with_options(tg, &output_host_path, &options)
				.await
				.wrap_err("Failed to check in the output.")?;

			tracing::info!(?artifact, "Checked in the process output.");

			// Verify the checksum if one was provided.
			if let Some(expected) = self.checksum.clone() {
				let actual = artifact
					.checksum(tg, expected.algorithm())
					.await
					.wrap_err("Failed to compute the checksum.")?;
				if expected != actual {
					return_error!(
						r#"The checksum did not match. Expected "{expected}" but got "{actual}"."#
					);
				}

				tracing::debug!("Validated the checksum.");
			}
			Value::Artifact(artifact)
		} else {
			Value::Null
		};

		Ok(value)
	}
}

fn root(context: Context) {
	unsafe {
		// Ask to receive a SIGKILL signal if the host process exits.
		let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
		if ret == -1 {
			abort_errno!("Failed to set PDEATHSIG.");
		}

		// Duplicate stdout and stderr to stderr.
		let ret = libc::dup2(libc::STDERR_FILENO, libc::STDOUT_FILENO);
		if ret == -1 {
			abort_errno!("Failed to duplicate stdout to the log.");
		}
		let ret = libc::dup2(libc::STDERR_FILENO, libc::STDERR_FILENO);
		if ret == -1 {
			abort_errno!("Failed to duplicate stderr to the log.");
		}

		// Close stdin.
		let ret = libc::close(libc::STDIN_FILENO);
		if ret == -1 {
			abort_errno!("Failed to close stdin.");
		}

		// If network access is disabled, set CLONE_NEWNET to isolate the guest's network namespace.
		let network_clone_flags = if context.network_enabled {
			0
		} else {
			libc::CLONE_NEWNET
		};

		// Spawn the guest process.
		let flags = libc::CLONE_NEWNS | libc::CLONE_NEWPID | network_clone_flags;
		let guest_process_pid = clone3(flags as _);
		if guest_process_pid == -1 {
			abort_errno!("Failed to spawn the guest process.");
		}
		if guest_process_pid == 0 {
			guest(&context);
		}

		// Send the guest process's PID to the host process, so the host process can write the UID and GID maps.
		let ret = libc::send(
			context.guest_socket.as_raw_fd(),
			std::ptr::addr_of!(guest_process_pid).cast(),
			std::mem::size_of_val(&guest_process_pid),
			0,
		);
		if ret == -1 {
			abort_errno!("Failed to send the PID of guest process.");
		}

		// Wait for the guest process.
		let mut status: libc::c_int = 0;
		let ret = libc::waitpid(guest_process_pid, &mut status, libc::__WALL);
		if ret == -1 {
			abort_errno!("Failed to wait for the guest process.");
		}
		let guest_process_exit_status = if libc::WIFEXITED(status) {
			let status = libc::WEXITSTATUS(status);
			ExitStatus::Code(status)
		} else if libc::WIFSIGNALED(status) {
			let signal = libc::WTERMSIG(status);
			ExitStatus::Signal(signal)
		} else {
			abort!("The guest process exited with neither a code nor a signal.");
		};

		// Send the host process the exit code of the guest process.
		let (kind, value) = match guest_process_exit_status {
			ExitStatus::Code(code) => (0u8, code),
			ExitStatus::Signal(signal) => (1, signal),
		};
		let ret = libc::send(
			context.guest_socket.as_raw_fd(),
			std::ptr::addr_of!(kind).cast(),
			std::mem::size_of_val(&kind),
			0,
		);
		if ret == -1 {
			abort_errno!("Failed to send the guest process's exit status's kind to the host.");
		}
		let ret = libc::send(
			context.guest_socket.as_raw_fd(),
			std::ptr::addr_of!(value).cast(),
			std::mem::size_of_val(&value),
			0,
		);
		if ret == -1 {
			abort_errno!("Failed to send the guest process's exit status's value to the host.");
		}

		std::process::exit(0)
	}
}

#[allow(clippy::too_many_lines)]
fn guest(context: &Context) {
	unsafe {
		// Ask to receive a SIGKILL signal if the host process exits.
		let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
		if ret == -1 {
			abort_errno!("Failed to set PDEATHSIG.");
		}

		// Wait for the notification from the host process to continue.
		let mut notification = 0u8;
		let ret = libc::recv(
			context.guest_socket.as_raw_fd(),
			std::ptr::addr_of_mut!(notification).cast(),
			std::mem::size_of_val(&notification),
			0,
		);
		if ret == -1 {
			abort_errno!("The guest process failed to receive the notification from the host process to continue.");
		}
		assert_eq!(notification, 1);

		// Perform the mounts.
		for mount in &context.mounts {
			let source = mount.source.as_ptr();
			let target = mount.target.as_ptr();
			let fstype = mount
				.fstype
				.as_ref()
				.map_or_else(std::ptr::null, |value| value.as_ptr());
			let flags = mount.flags;
			let data = mount
				.data
				.as_ref()
				.map_or_else(std::ptr::null, Vec::as_ptr)
				.cast();
			let ret = libc::mount(source, target, fstype, flags, data);
			if ret == -1 {
				abort_errno!(
					r#"Failed to mount "{}" to "{}"."#,
					mount.source.to_str().unwrap(),
					mount.target.to_str().unwrap(),
				);
			}
			if mount.readonly {
				let ret = libc::mount(
					source,
					target,
					fstype,
					flags | libc::MS_RDONLY | libc::MS_REMOUNT,
					data,
				);
				if ret == -1 {
					abort_errno!(
						r#"Failed to mount "{}" to "{}"."#,
						mount.source.to_str().unwrap(),
						mount.target.to_str().unwrap(),
					);
				}
			}
		}

		// Mount the root.
		let ret = libc::mount(
			context.root_host_path.as_ptr(),
			context.root_host_path.as_ptr(),
			std::ptr::null(),
			libc::MS_BIND | libc::MS_PRIVATE | libc::MS_REC,
			std::ptr::null(),
		);
		if ret == -1 {
			abort_errno!("Failed to mount the root.");
		}

		// Change the working directory to the pivoted root.
		let ret = libc::chdir(context.root_host_path.as_ptr());
		if ret == -1 {
			abort_errno!("Failed to change directory to the root.");
		}

		// Pivot the root.
		let ret = libc::syscall(libc::SYS_pivot_root, b".\0".as_ptr(), b".\0".as_ptr());
		if ret == -1 {
			abort_errno!("Failed to pivot the root.");
		}

		// Unmount the root.
		let ret = libc::umount2(b".\0".as_ptr().cast(), libc::MNT_DETACH);
		if ret == -1 {
			abort_errno!("Failed to unmount the root.");
		}

		// Remount the root as read-only.
		let ret = libc::mount(
			std::ptr::null(),
			b"/\0".as_ptr().cast(),
			std::ptr::null(),
			libc::MS_BIND | libc::MS_PRIVATE | libc::MS_RDONLY | libc::MS_REC | libc::MS_REMOUNT,
			std::ptr::null(),
		);
		if ret == -1 {
			abort_errno!("Failed to remount the root as read-only.");
		}

		// Set the working directory.
		let ret = libc::chdir(context.working_directory_guest_path.as_ptr());
		if ret == -1 {
			abort_errno!("Failed to set the working directory.");
		}

		// Exec.
		libc::execve(
			context.executable.as_ptr(),
			context.argv.as_ptr().cast(),
			context.envp.as_ptr().cast(),
		);
		abort_errno!(r#"Failed to call execve."#);
	}
}

/// Shared context between the host, root, and guest processes.
struct Context {
	/// The args.
	argv: CStringVec,

	/// The env.
	envp: CStringVec,

	/// The executable.
	executable: CString,

	/// The file descriptor of the guest side of the socket.
	guest_socket: std::os::unix::net::UnixStream,

	/// The mounts.
	mounts: Vec<Mount>,

	/// Whether to enable the network.
	network_enabled: bool,

	/// The host path to the root.
	root_host_path: CString,

	/// The guest path to the working directory.
	working_directory_guest_path: CString,
}

unsafe impl Send for Context {}

struct Mount {
	source: CString,
	target: CString,
	fstype: Option<CString>,
	flags: libc::c_ulong,
	data: Option<Vec<u8>>,
	readonly: bool,
}

struct CStringVec {
	_strings: Vec<CString>,
	pointers: Vec<*const libc::c_char>,
}

impl CStringVec {
	pub fn new(strings: Vec<CString>) -> Self {
		let mut pointers = strings.iter().map(|string| string.as_ptr()).collect_vec();
		pointers.push(std::ptr::null());
		Self {
			_strings: strings,
			pointers,
		}
	}

	pub fn as_ptr(&self) -> *const libc::c_char {
		self.pointers.as_ptr().cast()
	}
}

unsafe impl Send for CStringVec {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ExitStatus {
	Code(i32),
	Signal(i32),
}

fn clone3(flags: u64) -> libc::pid_t {
	let mut args = libc::clone_args {
		flags,
		stack: 0,
		stack_size: 0,
		pidfd: 0,
		child_tid: 0,
		parent_tid: 0,
		exit_signal: 0,
		tls: 0,
		set_tid: 0,
		set_tid_size: 0,
		cgroup: 0,
	};

	let return_code = unsafe {
		libc::syscall(
			libc::SYS_clone3,
			(&mut args) as *mut _,
			std::mem::size_of::<libc::clone_args>(),
		)
	};

	return_code
		.try_into()
		.unwrap_or(-1)
}

macro_rules! abort {
	($($t:tt)*) => {{
		eprintln!("Error: {}", format_args!($($t)*));
		std::process::exit(1)
	}};
}
use abort;

macro_rules! abort_errno {
	($($t:tt)*) => {{
		eprintln!("Error: {}", format_args!($($t)*));
		eprintln!("\t{}", std::io::Error::last_os_error());
		std::process::exit(1)
	}};
}
use abort_errno;
