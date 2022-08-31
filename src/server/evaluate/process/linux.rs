use crate::server::Server;
use anyhow::Result;
use libc::*;
use std::{
	collections::BTreeMap,
	ffi::CString,
	os::unix::prelude::OsStrExt,
	path::{Path, PathBuf},
	sync::Arc,
};

impl Server {
	pub(super) async fn run_linux_process(
		self: &Arc<Self>,
		envs: BTreeMap<String, String>,
		command: &Path,
		args: Vec<String>,
	) -> Result<()> {
		unsafe {
			let server_path = self.path().to_owned();

			// Create a temp for the chroot.
			let temp = self.create_temp().await?;
			let parent_child_root_path = self.temp_path(&temp);
			tokio::fs::create_dir(&parent_child_root_path).await?;

			// Create a socket pair so the child can communicate with the parent.
			let mut sockets = [0i32; 2];
			let ret = socketpair(AF_LOCAL, SOCK_SEQPACKET, 0, sockets.as_mut_ptr());
			assert!(ret == 0);
			let [parent_socket, child_socket] = sockets;

			let mut process = tokio::process::Command::new(command);
			process.env_clear();
			process.envs(envs);
			process.args(args);
			process.pre_exec(move || {
				// Unshare the user namespace.
				let ret = unshare(CLONE_NEWUSER);
				assert!(ret == 0);

				// Send the message to the parent process that the UID and GID maps are ready to be set.
				let pid = getpid();
				let message = pid.to_le_bytes();
				let ret = write(
					child_socket,
					message.as_ptr() as *const c_void,
					message.len(),
				);
				assert!(ret != -1);

				// Wait for the message from the parent process that the UID and GID maps have been set.
				let mut message = [0u8; 1];
				let ret = read(
					child_socket,
					message.as_mut_ptr() as *mut c_void,
					message.len(),
				);
				assert!(ret != -1);

				// Set the UID and GID.
				let uid = 0;
				let gid = 0;
				let ret = setresuid(uid, uid, uid);
				assert!(ret == 0);
				let ret = setresgid(gid, gid, gid);
				assert!(ret == 0);

				// Unshare the mount namespace.
				let ret = unshare(CLONE_NEWNS);
				assert!(ret == 0);

				// Ensure the parent child root path does not have shared propogation.
				let child_root_path = PathBuf::from("/");
				let child_root_path_c_string =
					CString::new(child_root_path.as_os_str().as_bytes()).unwrap();
				let ret = mount(
					std::ptr::null(),
					child_root_path_c_string.as_ptr(),
					std::ptr::null(),
					MS_REC | MS_PRIVATE,
					std::ptr::null(),
				);
				assert!(ret == 0);

				// Ensure the parent child root path is a mount point.
				let parent_child_root_path_c_string =
					CString::new(parent_child_root_path.as_os_str().as_bytes()).unwrap();
				let ret = mount(
					parent_child_root_path_c_string.as_ptr(),
					parent_child_root_path_c_string.as_ptr(),
					std::ptr::null(),
					MS_BIND,
					std::ptr::null(),
				);
				assert!(ret == 0);

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
				let parent_target_c_string =
					CString::new(parent_target_path.as_os_str().as_bytes()).unwrap();
				let ret = mount(
					parent_source_path_c_string.as_ptr(),
					parent_target_c_string.as_ptr(),
					std::ptr::null(),
					MS_BIND,
					std::ptr::null(),
				);
				assert!(ret == 0);

				// Pivot the root.
				let parent_child_root_path_c_string =
					CString::new(parent_child_root_path.as_os_str().as_bytes()).unwrap();
				let parent_parent_mount_path_c_string =
					CString::new(parent_parent_mount_path.as_os_str().as_bytes()).unwrap();
				let ret = syscall(
					SYS_pivot_root,
					parent_child_root_path_c_string.as_ptr(),
					parent_parent_mount_path_c_string.as_ptr(),
				);
				assert!(ret == 0);

				// Change the current directory to the child root path.
				let child_root_path = PathBuf::from("/");
				let child_root_path_c_string =
					CString::new(child_root_path.as_os_str().as_bytes()).unwrap();
				let ret = chdir(child_root_path_c_string.as_ptr());
				assert!(ret == 0);

				// Unmount the parent's root.
				let child_parent_mount_path_c_string =
					CString::new(child_parent_mount_path.as_os_str().as_bytes()).unwrap();
				let ret = umount2(child_parent_mount_path_c_string.as_ptr(), MNT_DETACH);
				assert!(ret == 0);

				// Remove the mountpoint for the parent's root.
				let ret = rmdir(child_parent_mount_path_c_string.as_ptr());
				assert!(ret == 0);

				Ok(())
			});

			// Spawn the child process.
			let spawn = tokio::task::spawn_blocking(move || {
				let child = process.spawn()?;
				Ok::<tokio::process::Child, anyhow::Error>(child)
			});

			// Wait for the message from the child process that the UID and GID maps are ready to be set.
			let mut message = [0u8; 4];
			let ret = read(
				parent_socket,
				message.as_mut_ptr() as *mut c_void,
				message.len(),
			);
			assert!(ret != -1);
			let pid = pid_t::from_le_bytes(message);

			// Write the UID map.
			let uid_map_path = PathBuf::from(format!("/proc/{pid}/uid_map"));
			let uid_map_path_c_string = CString::new(uid_map_path.as_os_str().as_bytes()).unwrap();
			let uid_map_fd = open(uid_map_path_c_string.as_ptr(), O_WRONLY);
			assert!(uid_map_fd != -1);
			let uid = getuid();
			let uid_map = format!("0 {} 1\n", uid);
			let ret = write(uid_map_fd, uid_map.as_ptr() as *const c_void, uid_map.len());
			assert_eq!(ret, uid_map.len() as isize);
			let ret = close(uid_map_fd);
			assert!(ret == 0);

			// Disable setgroups.
			let setgroups_path = PathBuf::from(format!("/proc/{pid}/setgroups"));
			let setgroups_path_c_string = CString::new(setgroups_path.as_os_str().as_bytes()).unwrap();
			let setgroups_fd = open(setgroups_path_c_string.as_ptr(), O_WRONLY);
			assert!(setgroups_fd != -1);
			let setgroups = "deny";
			let ret = write(
				setgroups_fd,
				setgroups.as_ptr() as *const c_void,
				setgroups.len(),
			);
			assert_eq!(ret, setgroups.len() as isize);
			let ret = close(setgroups_fd);
			assert!(ret == 0);

			// Write the GID map.
			let gid_map_path = PathBuf::from(format!("/proc/{pid}/gid_map"));
			let gid_map_path_c_string = CString::new(gid_map_path.as_os_str().as_bytes()).unwrap();
			let gid_map_fd = open(gid_map_path_c_string.as_ptr(), O_WRONLY);
			assert!(gid_map_fd != -1);
			let gid = getgid();
			let gid_map = format!("0 {} 1\n", gid);
			let ret = write(gid_map_fd, gid_map.as_ptr() as *const c_void, gid_map.len());
			assert_eq!(ret, gid_map.len() as isize);
			let ret = close(gid_map_fd);
			assert!(ret == 0);

			// Send the message to the child process that the UID and GID maps have been set.
			let message = [0u8; 1];
			let ret = write(
				parent_socket,
				message.as_ptr() as *const c_void,
				message.len(),
			);
			assert!(ret != -1);

			// Wait for the child process to exit.
			let mut child = spawn.await.unwrap()?;
			child.wait().await?;

			Ok(())
		}
	}
}
