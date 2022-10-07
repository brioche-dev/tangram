use crate::{
	expression::{self, Expression},
	hash::Hash,
	system::System,
};

use super::{SandboxPathMode, SandboxedCommand};
use anyhow::{anyhow, bail, Context, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use libc::*;
use std::{
	ffi::CString,
	os::unix::prelude::OsStrExt,
	path::{Path, PathBuf},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl SandboxedCommand {
	pub async fn run(self) -> Result<()> {
		// Create a temp path for the chroot.
		let temp_path = self.builder.create_temp_path();

		// Create the chroot directory.
		let parent_child_root_path = temp_path;
		tokio::fs::create_dir(&parent_child_root_path)
			.await
			.context("Failed to create the chroot directory.")?;

		// Create a symlink from /bin/sh in the chroot to a fragment with statically-linked bash.
		let bash_artifact = self
			.bash_artifact()
			.await
			.context("Failed to evaluate the bash artifact.")?;
		let bash_checkout = self
			.builder
			.checkout_to_artifacts(bash_artifact)
			.await
			.context("Failed to create the bash artifact checkout.")?;
		tokio::fs::create_dir(parent_child_root_path.join("bin")).await?;
		tokio::fs::symlink(
			self.builder
				.artifacts_path()
				.join(&bash_checkout)
				.join("bin/bash"),
			parent_child_root_path.join("bin/sh"),
		)
		.await?;

		// Create a socket pair so the parent and child can communicate to set up the sandbox.
		let (mut parent_socket, child_socket) =
			tokio::net::UnixStream::pair().context("Failed to create socket pair.")?;
		let mut child_socket = child_socket
			.into_std()
			.context("Failed to convert the child socket to std.")?;
		child_socket
			.set_nonblocking(false)
			.context("Failed to make the child socket nonblocking.")?;

		// Create the process.
		let mut process = tokio::process::Command::new(&self.command.string);

		// Set the working directory.
		process.current_dir(&self.working_dir);

		// Set the envs.
		process.env_clear();
		process.envs(self.envs.iter().map(|(k, v)| (k.clone(), v.string.clone())));

		// Set the args.
		process.args(self.args.iter().map(|arg| arg.string.clone()));

		// Set up the sandbox.
		unsafe {
			process.pre_exec(move || {
				pre_exec(&mut child_socket, &parent_child_root_path, &self)
					.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
			})
		};

		// Spawn the process.
		let spawn_task = tokio::task::spawn_blocking(move || {
			let child = process.spawn().context("Failed to spawn the process.")?;
			Ok::<_, anyhow::Error>(child)
		});

		// Wait for the message from the child process that the UID and GID maps are ready to be set.
		let pid: pid_t = parent_socket
			.read_i32()
			.await
			.context("Failed to read from the parent socket.")?;

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

	async fn bash_artifact(&self) -> Result<Hash> {
		// Get the URL and hash for the system.
		let (url, hash) = match self.system {
			System::Amd64Linux => (
				"https://github.com/tangramdotdev/bootstrap/releases/download/v2022.10.07/bash_static_x86_64_20221007.tar.zstd",
				"9217dd76ac03ef36763a1964751f1eb681f8d7540552455f4108685e15f179fc",
			),
			System::Arm64Linux => (
				"https://github.com/tangramdotdev/bootstrap/releases/download/v2022.10.07/bash_static_aarch64_20221007.tar.zstd",
				"c1a4dbc54fb8cd6cf5d026d3094577842208f39e872453e6e2445489a86c9da9",
			),
			_ => bail!(r#"Unexpected system "{}"."#, self.system),
		};

		// Create the expression.
		let hash = self
			.builder
			.add_expression(&Expression::Fetch(expression::Fetch {
				url: url.parse().unwrap(),
				hash: Some(hash.parse().unwrap()),
				unpack: true,
			}))
			.await
			.context("Failed to add the bash expression.")?;

		// Evaluate the expression.
		let output_hash = self
			.builder
			.evaluate(hash, self.parent_hash)
			.await
			.context("Failed to evaluate the expression.")?;

		Ok(output_hash)
	}
}

fn pre_exec(
	child_socket: &mut std::os::unix::net::UnixStream,
	parent_child_root_path: &Path,
	command: &SandboxedCommand,
) -> Result<()> {
	// Unshare the user namespace.
	let ret = unsafe { unshare(CLONE_NEWUSER) };
	if ret != 0 {
		bail!(anyhow!(std::io::Error::last_os_error())
			.context("Failed to unshare the user namepsace."));
	}

	// Send the message to the parent process that the UID and GID maps are ready to be set.
	let pid = unsafe { getpid() };
	child_socket.write_i32::<BigEndian>(pid).context("Failed to send the message to the parent process that the UID and GID maps are ready to be set.")?;

	// Wait for the message from the parent process that the UID and GID maps have been set.
	child_socket.read_u8().context("Failed to receive the message from the parent process that the UID and GID maps have been set.")?;

	// Set the UID and GID.
	let uid = 0;
	let ret = unsafe { setresuid(uid, uid, uid) };
	if ret != 0 {
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to set the uid."));
	}
	let gid = 0;
	let ret = unsafe { setresgid(gid, gid, gid) };
	if ret != 0 {
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to set the gid."));
	}

	// Unshare the mount namespace.
	let ret = unsafe { unshare(CLONE_NEWNS) };
	if ret != 0 {
		bail!(
			anyhow!(std::io::Error::last_os_error()).context("Failed to unshare mount namespace.")
		);
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
		bail!(anyhow!(std::io::Error::last_os_error())
			.context("Failed to ensure the new root does not have shared propagation."));
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
		bail!(anyhow!(std::io::Error::last_os_error())
			.context("Failed to ensure the new root is a mount point."));
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
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to mount /proc."));
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
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to mount /dev."));
	}

	// Create /tmp.
	let child_tmp_path = parent_child_root_path.join("tmp");
	std::fs::create_dir_all(&child_tmp_path)?;
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
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to create /tmp."));
	}

	// Create /etc.
	let child_etc_path = parent_child_root_path.join("etc");
	std::fs::create_dir_all(&child_etc_path)?;

	if command.enable_network_access {
		// Copy resolv.conf to re-use DNS config from host.
		std::fs::copy("/etc/resolv.conf", child_etc_path.join("resolv.conf"))?;
	} else {
		// Unshare the network namespace to disable network access.
		let ret = unsafe { unshare(CLONE_NEWNET) };
		if ret != 0 {
			bail!(anyhow!(std::io::Error::last_os_error())
				.context("Failed to unshare network namespace."));
		}
	}

	// Mount all paths used in the build.
	for (path, mode) in command.paths() {
		let parent_source_path = match mode {
			// Allow access to just the path.
			SandboxPathMode::Read | SandboxPathMode::ReadWrite => &path,
			// To allow creation of the new file, allow access to the parent path.
			SandboxPathMode::ReadWriteCreate => path.parent().unwrap_or(&path),
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
			bail!(anyhow!(std::io::Error::last_os_error())
				.context("Failed to mount the builder path."));
		}

		// Remount as read-only if writes are disabled. Bind mounts can only be made read-only by mounting it normally then remounting in read-only mode.
		match mode {
			SandboxPathMode::Read => {
				let ret = unsafe {
					mount(
						parent_source_path_c_string.as_ptr(),
						parent_target_c_string.as_ptr(),
						std::ptr::null(),
						MS_REMOUNT | MS_BIND | MS_RDONLY,
						std::ptr::null(),
					)
				};
				if ret != 0 {
					bail!(anyhow!(std::io::Error::last_os_error())
						.context("Failed to re-mount the builder path as read-only."));
				}
			},
			SandboxPathMode::ReadWrite | SandboxPathMode::ReadWriteCreate => {},
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
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to pivot the root."));
	}

	// Change the current directory to the child root path.
	let child_root_path = PathBuf::from("/");
	let child_root_path_c_string = CString::new(child_root_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe { chdir(child_root_path_c_string.as_ptr()) };
	if ret != 0 {
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to chdir."));
	}

	// Unmount the parent's root.
	let child_parent_mount_path_c_string =
		CString::new(child_parent_mount_path.as_os_str().as_bytes()).unwrap();
	let ret = unsafe { umount2(child_parent_mount_path_c_string.as_ptr(), MNT_DETACH) };
	if ret != 0 {
		bail!(anyhow!(std::io::Error::last_os_error())
			.context("Failed to unmount the parent's root."));
	}

	// Remove the mountpoint for the parent's root.
	let ret = unsafe { rmdir(child_parent_mount_path_c_string.as_ptr()) };
	if ret != 0 {
		bail!(anyhow!(std::io::Error::last_os_error())
			.context("Failed to remove the mountpoint for the parent's root."));
	}

	Ok(())
}
