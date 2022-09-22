use super::Process;
use crate::{
	expression::{self, Expression},
	hash::Hash,
	server::Server,
	system::System,
};
use anyhow::{anyhow, bail, Context, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use libc::*;
use std::{
	collections::BTreeMap,
	ffi::CString,
	os::unix::prelude::OsStrExt,
	path::{Path, PathBuf},
	sync::Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

impl Process {
	pub(super) async fn run_linux_process(
		&self,
		server: &Arc<Server>,
		system: System,
		envs: BTreeMap<String, String>,
		command: PathBuf,
		args: Vec<String>,
		parent_hash: Hash,
	) -> Result<()> {
		let server_path = server.path().to_owned();

		// Create a temp for the chroot.
		let temp = server.create_temp();

		// Create the chroot directory.
		let parent_child_root_path = server.temp_path(&temp);
		tokio::fs::create_dir(&parent_child_root_path)
			.await
			.context("Failed to create the chroot directory.")?;

		// Create a symlink from /bin/sh in the chroot to a fragment with statically-linked bash.
		let bash_artifact = self
			.bash_artifact(server, system, parent_hash)
			.await
			.context("Failed to evaluate the bash artifact.")?;
		let bash_fragment = server
			.create_fragment(bash_artifact)
			.await
			.context("Failed to create the bash fragment.")?;
		tokio::fs::create_dir(parent_child_root_path.join("bin")).await?;
		tokio::fs::symlink(
			server.fragment_path(&bash_fragment).join("bin/bash"),
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
		let mut process = tokio::process::Command::new(command);

		// Set the envs.
		process.env_clear();
		process.envs(envs);

		// Set the args.
		process.args(args);

		// Set up the sandbox.
		unsafe {
			process.pre_exec(move || {
				pre_exec(&mut child_socket, &parent_child_root_path, &server_path)
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

	async fn bash_artifact(
		&self,
		server: &Arc<Server>,
		system: System,
		parent_hash: Hash,
	) -> Result<Hash> {
		// Get the URL and hash for the system.
		let (url, hash) = match system {
			System::Amd64Linux => (
				"https://github.com/tangramdotdev/bootstrap/releases/download/v0.1/bash_static_x86_64_20220907.tar.zstd",
				"d81feb117184f1260dee13ccade1892e924b56f8280620fe1e91ab8ea546c853",
			),
			System::Arm64Linux => (
				"https://github.com/tangramdotdev/bootstrap/releases/download/v0.1/bash_static_aarch64_20220907.tar.zstd",
				"1eef7627df44f6f2708a9ca7340f501dfabfa4e4f58c4d95b12ba8290719dbda",
			),
			_ => bail!(r#"Unexpected system "{}"."#, system),
		};

		// Create the expression.
		let hash = server
			.add_expression(&Expression::Fetch(expression::Fetch {
				url: url.parse().unwrap(),
				hash: Some(hash.parse().unwrap()),
				unpack: true,
			}))
			.await
			.context("Failed to add the bash expression.")?;

		// Evaluate the expression.
		let output_hash = server
			.evaluate(hash, parent_hash)
			.await
			.context("Failed to evaluate the expression.")?;

		Ok(output_hash)
	}
}

fn pre_exec(
	child_socket: &mut std::os::unix::net::UnixStream,
	parent_child_root_path: &Path,
	server_path: &Path,
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

	// Mount the server path.
	let parent_source_path = &server_path;
	let parent_source_path_c_string =
		CString::new(parent_source_path.as_os_str().as_bytes()).unwrap();
	let parent_target_path =
		parent_child_root_path.join(parent_source_path.strip_prefix("/").unwrap());
	std::fs::create_dir_all(&parent_target_path).unwrap();
	let parent_target_c_string = CString::new(parent_target_path.as_os_str().as_bytes()).unwrap();
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
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to mount the server path."));
	}

	// Mount /proc
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
			MS_BIND,
			std::ptr::null(),
		)
	};
	if ret != 0 {
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to bindmount /proc"));
	}

	// Mount /dev
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
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to bindmount /dev"));
	}

	// Create <chroot>/tmp
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
		bail!(anyhow!(std::io::Error::last_os_error()).context("Failed to create /tmp"));
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
