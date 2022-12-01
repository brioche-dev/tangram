#![allow(clippy::similar_names)]

use super::{PathMode, ReferencedPathSet};
use crate::builder::State;
use anyhow::{bail, Context, Result};
use bstr::ByteSlice;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use libc::{
	c_ulong, chdir, getgid, getpid, getuid, mount, rmdir, setresgid, setresuid, syscall, umount2,
	unshare, SYS_pivot_root, CLONE_NEWNET, CLONE_NEWNS, CLONE_NEWUSER, MNT_DETACH, MS_BIND,
	MS_LAZYTIME, MS_MANDLOCK, MS_NOATIME, MS_NODEV, MS_NODIRATIME, MS_NOEXEC, MS_NOSUID,
	MS_PRIVATE, MS_RDONLY, MS_REC, MS_RELATIME, MS_REMOUNT, MS_STRICTATIME,
};
use std::{
	collections::BTreeMap,
	ffi::CString,
	io::BufRead,
	os::unix::prelude::OsStrExt,
	path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl State {
	pub async fn run_process_linux(
		&self,
		working_directory: String,
		env: BTreeMap<String, String>,
		command: String,
		args: Vec<String>,
		referenced_path_set: ReferencedPathSet,
		network: bool,
	) -> Result<()> {
		// Create a temp path for the chroot.
		let parent_child_root_path = self.create_temp_path();
		tokio::fs::create_dir_all(&parent_child_root_path).await?;

		// Create a socket pair so the parent and child can communicate to set up the sandbox.
		let (mut parent_socket, child_socket) =
			tokio::net::UnixStream::pair().context("Failed to create socket pair.")?;

		// Convert the child socket to the std equivalent and make it blocking so it can be used in `pre_exec`.
		let mut child_socket = child_socket
			.into_std()
			.context("Failed to convert the child socket to std.")?;
		child_socket
			.set_nonblocking(false)
			.context("Failed to make the child socket nonblocking.")?;

		// Create the process.
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
				pre_exec(
					&mut child_socket,
					&parent_child_root_path,
					&referenced_path_set,
					network,
				)
				.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
			})
		};

		// Spawn the process.
		let spawn_task = tokio::task::spawn_blocking(move || {
			let child = command.spawn().context("Failed to spawn the process.")?;
			Ok::<_, anyhow::Error>(child)
		});

		// Wait for the message from the child process with the child process's pid.
		let pid = match parent_socket.read_i32().await {
			Ok(pid) => Some(pid),

			// If the child process did not respond with the pid then continue.
			Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => None,

			Err(error) => {
				return Err(error).context("Failed to read from the parent socket.");
			},
		};

		if let Some(pid) = pid {
			// Write the UID map.
			let uid_map_path = PathBuf::from(format!("/proc/{pid}/uid_map"));
			let uid = unsafe { getuid() };
			let uid_map = format!("0 {uid} 1\n");
			tokio::fs::write(&uid_map_path, &uid_map)
				.await
				.context("Failed to write the UID map file.")?;

			// Disable setgroups.
			let setgroups_path = PathBuf::from(format!("/proc/{pid}/setgroups"));
			let setgroups = "deny";
			tokio::fs::write(&setgroups_path, &setgroups)
				.await
				.context("Failed to write the setgroups file.")?;

			// Write the GID map.
			let gid_map_path = PathBuf::from(format!("/proc/{pid}/gid_map"));
			let gid = unsafe { getgid() };
			let gid_map = format!("0 {gid} 1\n");
			tokio::fs::write(&gid_map_path, &gid_map)
				.await
				.context("Failed to write the GID map file.")?;

			// Send the message to the child process that the UID and GID maps have been set.
			parent_socket.write_u8(0).await.context(
				"Failed to notify the child process that the UID and GID maps have been set.",
			)?;
		}

		// Wait for the sandbox parent task to complete.
		let mut child = spawn_task
			.await
			.unwrap()
			.context("The spawn task failed.")?;

		// Wait for the child process to exit.
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

fn pre_exec(
	child_socket: &mut std::os::unix::net::UnixStream,
	parent_child_root_path: &Path,
	referenced_path_set: &ReferencedPathSet,
	network: bool,
) -> std::io::Result<()> {
	let result = set_up_sandbox(
		child_socket,
		parent_child_root_path,
		referenced_path_set,
		network,
	);
	match result {
		Ok(()) => Ok(()),
		Err(SandboxError::Incomplete(error)) => {
			// Print a warning if the sandbox setup did not finish completely.
			eprintln!("Warning: Sandbox setup failed. {error:#}");
			Ok(())
		},
		Err(error) => {
			// Print the error and bail if sandbox setup returned a fatal error. We print it here because the failures in `pre_exec` always shows up as `EINVAL` to the caller if there's no OS error number associated with it.
			eprintln!("An error occurred while trying to setup the sandbox. {error:#}");
			Err(std::io::Error::new(std::io::ErrorKind::Other, error))
		},
	}
}

#[derive(Debug, thiserror::Error)]
enum SandboxIncomplete {
	#[error("Failed to unshare the user namespace.")]
	UnshareUser(#[source] std::io::Error),

	#[error("Failed to set the uid.")]
	SetUidFailed(#[source] std::io::Error),

	#[error("Failed to set the gid.")]
	SetGidFailed(#[source] std::io::Error),

	#[error("Failed to unshare the mount namespace.")]
	UnshareMount(#[source] std::io::Error),

	#[error("Failed to ensure the root does not have shared propagation.")]
	MountPrivateRoot(#[source] std::io::Error),

	#[error("Failed to ensure the new root is a mount point.")]
	MountBindRoot(#[source] std::io::Error),

	#[error("Failed to mount {1:?}.")]
	MountFailed(#[source] std::io::Error, PathBuf),

	#[error("Failed to mount {1:?} as read-only.")]
	MountReadOnlyFailed(#[source] std::io::Error, PathBuf),

	#[error("Failed to pivot the root.")]
	PivotRootFailed(#[source] std::io::Error),

	#[error("Failed to chdir.")]
	ChdirFailed(#[source] std::io::Error),

	#[error("Failed to unmount the parent's root.")]
	UnmountParentFailed(#[source] std::io::Error),

	#[error("Failed to remove the mountpoint for the parent's root.")]
	RemoveParentFailed(#[source] std::io::Error),

	#[error("Failed to unshare the network namespace.")]
	UnshareNetwork(#[source] std::io::Error),

	#[error("Failed to get resolv.conf from host.")]
	ResolvConf(#[source] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
enum SandboxError {
	#[error("Failed to send the message to the parent process that the UID and GID maps are ready to be set.")]
	FailedToSendUidMapMessage(#[source] std::io::Error),

	#[error("Failed to receive the message from the parent process that the UID and GID maps have been set.")]
	FailedToReceiveUidMapMessage(#[source] std::io::Error),

	#[error("Failed to get options for mountpoint {1:?}.")]
	FailedToGetMountpointOptions(#[source] anyhow::Error, PathBuf),

	#[error("Failed to set up sandbox.")]
	Incomplete(#[from] SandboxIncomplete),
}

#[allow(clippy::too_many_lines)]
fn set_up_sandbox(
	child_socket: &mut std::os::unix::net::UnixStream,
	parent_child_root_path: &Path,
	referenced_path_set: &ReferencedPathSet,
	network: bool,
) -> Result<(), SandboxError> {
	// Unshare the user namespace.
	let ret = unsafe { unshare(CLONE_NEWUSER) };
	if ret != 0 {
		return Err(SandboxIncomplete::UnshareUser(std::io::Error::last_os_error()).into());
	}

	// Send the message to the parent process that the UID and GID maps are ready to be set.
	let pid = unsafe { getpid() };
	child_socket
		.write_i32::<BigEndian>(pid)
		.map_err(SandboxError::FailedToSendUidMapMessage)?;

	// Wait for the message from the parent process that the UID and GID maps have been set.
	child_socket
		.read_u8()
		.map_err(SandboxError::FailedToReceiveUidMapMessage)?;

	// Set the UID and GID.
	let uid = 0;
	let ret = unsafe { setresuid(uid, uid, uid) };
	if ret != 0 {
		return Err(SandboxIncomplete::SetUidFailed(std::io::Error::last_os_error()).into());
	}
	let gid = 0;
	let ret = unsafe { setresgid(gid, gid, gid) };
	if ret != 0 {
		return Err(SandboxIncomplete::SetGidFailed(std::io::Error::last_os_error()).into());
	}

	// Unshare the mount namespace.
	let ret = unsafe { unshare(CLONE_NEWNS) };
	if ret != 0 {
		return Err(SandboxIncomplete::UnshareMount(std::io::Error::last_os_error()).into());
	}

	// Ensure the parent child root path does not have shared propagation.
	let child_root_path = PathBuf::from("/");
	let child_root_path_c_string = CString::new(child_root_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe {
		mount(
			std::ptr::null(),
			child_root_path_c_string.as_ptr(),
			std::ptr::null(),
			MS_REC | MS_PRIVATE,
			std::ptr::null(),
		)
	};
	if ret != 0 {
		return Err(SandboxIncomplete::MountPrivateRoot(std::io::Error::last_os_error()).into());
	}

	// Ensure the parent child root path is a mount point.
	let parent_child_root_path_c_string =
		CString::new(parent_child_root_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe {
		mount(
			parent_child_root_path_c_string.as_ptr(),
			parent_child_root_path_c_string.as_ptr(),
			std::ptr::null(),
			MS_BIND,
			std::ptr::null(),
		)
	};	
	if ret != 0 {
		return Err(SandboxIncomplete::MountBindRoot(std::io::Error::last_os_error()).into());
	}

	// Create the parent mount path.
	let child_parent_mount_path = PathBuf::from("/parent");
	let parent_parent_mount_path =
		parent_child_root_path.join(child_parent_mount_path.strip_prefix("/").unwrap());
	std::fs::create_dir_all(&parent_parent_mount_path).unwrap();

	// Mount /proc.
	let parent_proc = PathBuf::from("/proc");
	let child_proc_path = parent_child_root_path.join("proc");
	std::fs::create_dir_all(&child_proc_path).unwrap();
	let parent_proc_c_string = CString::new(parent_proc.as_os_str().as_bytes()).unwrap();
	let child_proc_path_c_string = CString::new(child_proc_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe {
		mount(
			parent_proc_c_string.as_ptr(),
			child_proc_path_c_string.as_ptr(),
			std::ptr::null(),
			MS_BIND | MS_REC,
			std::ptr::null(),
		)
	};
	if ret != 0 {
		return Err(
			SandboxIncomplete::MountFailed(std::io::Error::last_os_error(), parent_proc).into(),
		);
	}

	// Mount /dev.
	let parent_dev = PathBuf::from("/dev");
	let child_dev_path = parent_child_root_path.join("dev");
	std::fs::create_dir_all(&child_dev_path).unwrap();
	let parent_dev_c_string = CString::new(parent_dev.as_os_str().as_bytes()).unwrap();
	let child_dev_path_c_string = CString::new(child_dev_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe {
		mount(
			parent_dev_c_string.as_ptr(),
			child_dev_path_c_string.as_ptr(),
			std::ptr::null(),
			MS_BIND | MS_REC,
			std::ptr::null(),
		)
	};
	if ret != 0 {
		return Err(
			SandboxIncomplete::MountFailed(std::io::Error::last_os_error(), parent_dev).into(),
		);
	}

	// Create /tmp.
	let child_tmp_path = parent_child_root_path.join("tmp");
	std::fs::create_dir_all(&child_tmp_path).unwrap();
	let child_tmp_path_c_string = CString::new(child_tmp_path.as_os_str().as_bytes()).unwrap();
	let tmpfs_c_string = CString::new("tmpfs").unwrap();
	let ret = unsafe {
		mount(
			tmpfs_c_string.as_ptr(),
			child_tmp_path_c_string.as_ptr(),
			tmpfs_c_string.as_ptr(),
			0,
			std::ptr::null(),
		)
	};
	if ret != 0 {
		return Err(SandboxIncomplete::MountFailed(
			std::io::Error::last_os_error(),
			PathBuf::from("/tmp"),
		)
		.into());
	}

	// Create /etc.
	let child_etc_path = parent_child_root_path.join("etc");
	std::fs::create_dir_all(&child_etc_path).unwrap();

	if network {
		// If network access is enabled, copy /etc/resolv.conf to use the DNS config from the host.
		std::fs::copy("/etc/resolv.conf", child_etc_path.join("resolv.conf"))
			.map_err(SandboxIncomplete::ResolvConf)?;
	} else {
		// If network access is disabled, unshare the network namespace.
		let ret = unsafe { unshare(CLONE_NEWNET) };
		if ret != 0 {
			return Err(SandboxIncomplete::UnshareNetwork(std::io::Error::last_os_error()).into());
		}
	}

	// Mount all referenced paths.
	for referenced_path in referenced_path_set {
		let parent_source_path = match referenced_path.mode {
			// Allow access to just the path.
			PathMode::Read | PathMode::ReadWrite => &referenced_path.path,
			// To allow creation of the new file, allow access to the parent path.
			PathMode::ReadWriteCreate => referenced_path
				.path
				.parent()
				.unwrap_or(&referenced_path.path),
		};
		let parent_source_path_c_string =
			CString::new(parent_source_path.as_os_str().as_bytes()).unwrap();
		let parent_target_path =
			parent_child_root_path.join(parent_source_path.strip_prefix("/").unwrap());
		std::fs::create_dir_all(&parent_target_path).unwrap();
		let parent_target_c_string =
			CString::new(parent_target_path.as_os_str().as_bytes()).unwrap();
		let ret = unsafe {
			mount(
				parent_source_path_c_string.as_ptr(),
				parent_target_c_string.as_ptr(),
				std::ptr::null(),
				MS_BIND,
				std::ptr::null(),
			)
		};
		if ret != 0 {
			return Err(SandboxIncomplete::MountFailed(
				std::io::Error::last_os_error(),
				parent_source_path.to_owned(),
			)
			.into());
		}

		// Remount as read-only if writes are disabled. Bind mounts can only be made read-only by mounting it normally then remounting in read-only mode.
		match referenced_path.mode {
			PathMode::Read => {
				let current_mount_options = get_mountpoint_remount_options(&parent_target_path)
					.map_err(|error| {
						SandboxError::FailedToGetMountpointOptions(
							error,
							parent_target_path.clone(),
						)
					})?;

				let ret = unsafe {
					mount(
						parent_source_path_c_string.as_ptr(),
						parent_target_c_string.as_ptr(),
						std::ptr::null(),
						MS_REMOUNT | MS_BIND | MS_RDONLY | current_mount_options,
						std::ptr::null(),
					)
				};
				if ret != 0 {
					return Err(SandboxIncomplete::MountReadOnlyFailed(
						std::io::Error::last_os_error(),
						parent_source_path.to_owned(),
					)
					.into());
				}
			},
			PathMode::ReadWrite | PathMode::ReadWriteCreate => {},
		}
	}

	// Pivot the root.
	let parent_child_root_path_c_string =
		CString::new(parent_child_root_path.as_os_str().as_bytes()).unwrap();
	let parent_parent_mount_path_c_string =
		CString::new(parent_parent_mount_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe {
		syscall(
			SYS_pivot_root,
			parent_child_root_path_c_string.as_ptr(),
			parent_parent_mount_path_c_string.as_ptr(),
		)
	};
	if ret != 0 {
		return Err(SandboxIncomplete::PivotRootFailed(std::io::Error::last_os_error()).into());
	}

	// Change the current directory to the child root path.
	let child_root_path = PathBuf::from("/");
	let child_root_path_c_string = CString::new(child_root_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe { chdir(child_root_path_c_string.as_ptr()) };
	if ret != 0 {
		return Err(SandboxIncomplete::ChdirFailed(std::io::Error::last_os_error()).into());
	}

	// Unmount the parent's root.
	let child_parent_mount_path_c_string =
		CString::new(child_parent_mount_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe { umount2(child_parent_mount_path_c_string.as_ptr(), MNT_DETACH) };
	if ret != 0 {
		return Err(SandboxIncomplete::UnmountParentFailed(std::io::Error::last_os_error()).into());
	}

	// Remove the mountpoint for the parent's root.
	let ret = unsafe { rmdir(child_parent_mount_path_c_string.as_ptr()) };
	if ret != 0 {
		return Err(SandboxIncomplete::RemoveParentFailed(std::io::Error::last_os_error()).into());
	}

	Ok(())
}

// Get the current options associated with a mountpoint that affect remounting. In some cases, when calling `mount` targeting an existing bind mount with the `MS_REMOUNT` option set, some extra options from the mountpoint underlying the bind mount need to be included too. Not including some extra options can cause the `mount` call to fail, such as the `MS_NODEV` option when inside a container. This function parses the mount table at `/proc/mount` to get the current mount options. See mount(2) for a list of mount options that are used when remounting.
fn get_mountpoint_remount_options(mountpoint: &Path) -> Result<c_ulong> {
	// Read the mount table for the current mount namespace. This file uses the same format as fstab.
	let mounts_table = std::fs::File::open("/proc/mounts")?;
	let mounts_table = std::io::BufReader::new(mounts_table);

	let mount_rows = mounts_table.split(b'\n');

	// Iterate through each row in the mount table until we find the row matching `mountpoint`.
	for mount_row in mount_rows {
		let mount_row = mount_row?;

		// Remove leading/trailing whitespace.
		let mount_row = mount_row.trim();

		// Ignore blank lines.
		if mount_row.is_empty() {
			continue;
		}

		// Ignore comments.
		if mount_row.starts_with_str("#") {
			continue;
		}

		// Break the line into fields separated by whitespace.
		let mut mount_fields = mount_row.fields();

		// Ignore the mount source (first field).
		let _mount_source = mount_fields.next();

		// Get the mount target and decode spaces/tabs (second field).
		let mount_target = mount_fields.next().unwrap_or_default().to_owned();
		let mount_target = mount_target.replace("\\040", " ").replace("\\011", "\t");
		let mount_target = mount_target
			.to_path()
			.context("Failed to parse mount target path.")?;

		// Skip this row if it isn't the mountpoint we're looking for
		if mountpoint != mount_target {
			continue;
		}

		// Ignore the filesystem type (third field).
		let _mount_fs_type = mount_fields.next();

		// Get the list of mount options (fourth field).
		let mount_opts = mount_fields.next().unwrap_or_default();

		// Gather a list of bitflags by looking for known mount options (comma separated).
		let mut mount_flags = 0;
		for mount_opt in mount_opts.split_str(",") {
			match mount_opt {
				b"ro" => {
					mount_flags |= MS_RDONLY;
				},
				b"noatime" => {
					mount_flags |= MS_NOATIME;
				},
				b"nodev" => {
					mount_flags |= MS_NODEV;
				},
				b"nodiratime" => {
					mount_flags |= MS_NODIRATIME;
				},
				b"noexec" => {
					mount_flags |= MS_NOEXEC;
				},
				b"mand" => {
					mount_flags |= MS_MANDLOCK;
				},
				b"relatime" => {
					mount_flags |= MS_RELATIME;
				},
				b"lazytime" => {
					mount_flags |= MS_LAZYTIME;
				},
				b"nosuid" => {
					mount_flags |= MS_NOSUID;
				},
				b"strictatime" => {
					mount_flags |= MS_STRICTATIME;
				},
				_ => {
					// Ignore other mount options.
				},
			}
		}

		return Ok(mount_flags);
	}

	bail!(r#"Could not find mountpoint "{}"."#, mountpoint.display());
}
