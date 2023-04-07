use super::{
	c_char, c_void, copy_file, crash, crash_errno, join_c, mkdir_p, socket_recv, write_to_file,
	SandboxContext, TANGRAM_GID, TANGRAM_UID,
};
use crate::{process::run, util::fs};
use std::{
	ffi::{CStr, OsStr},
	io::ErrorKind,
	os::unix::ffi::OsStrExt,
};

pub extern "C" fn inner_child_callback(arg: *mut c_void) -> libc::c_int {
	let ctx = unsafe { &mut *(arg as *mut SandboxContext) };
	unsafe { inner_child_main(ctx) }
}

unsafe fn inner_child_main(ctx: &mut SandboxContext) -> i32 {
	// Ask to be SIGKILLed if the parent process exits.
	let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
	if ret == -1 {
		crash_errno!("Failed to set PDEATHSIG.");
	}

	// Wait for the OK to exec the process.
	let Ok(ok_to_continue) = socket_recv::<u8>(ctx.coordination_socket_inner_fd) else {
		crash!("Inner child failed to receive OK to continue from coordination socket.");
	};
	assert_eq!(ok_to_continue, 1);

	// Configure mounts.
	populate_chroot_with_mounts_from_context(ctx);

	// Mount /dev and /proc.
	mount_dev_proc_tmp(ctx);

	// Create /etc and populate it with some configuration.
	create_etc_and_populate(ctx);

	// Pivot root to the guest chroot.
	pivot_root_to_chroot_and_remove_old_root(ctx);

	// Remount the root as read-only.
	remount_root_as_read_only(ctx);

	// Set the working directory.
	set_working_directory(ctx);

	// Spawn the subprocess.
	libc::execve(
		ctx.executable.as_ptr(),
		ctx.argv.as_ptr(),
		ctx.envp.as_ptr(),
	);
	crash_errno!(r#"Failed to exec subprocess "{:?}"."#, ctx.executable);
}

unsafe fn populate_chroot_with_mounts_from_context(ctx: &SandboxContext) {
	for mount in ctx.mounts {
		// Format the host's path to the mount destination.
		let mut target_path_buf = [0u8; libc::PATH_MAX as usize];
		let target_path = join_c!(
			&mut target_path_buf,
			ctx.host_chroot_path,
			"/",
			mount.guest_path.as_os_str().as_bytes()
		);

		// Null-terminate the source path.
		let mut source_path_buf = [0u8; libc::PATH_MAX as usize];
		let source_path = join_c!(&mut source_path_buf, mount.host_path.as_os_str().as_bytes());

		// Create the mountpoint's parent directory recursively, if it doesn't already exist.
		let parent_dir_os = OsStr::from_bytes(target_path.to_bytes());
		let parent_dir = fs::Path::new(parent_dir_os);
		if let Some(parent) = parent_dir.parent() {
			if let Err(e) = mkdir_p(&parent.as_os_str().as_bytes()) {
				crash!("Failed to make mountpoint parent dir: {}", e);
			}
		}

		// Create a mountpoint, if one doesn't already exist.
		match mount.kind {
			run::Kind::File => {
				// Touch a file if it doesn't already exist.
				let ret = libc::creat(target_path.as_ptr(), 0o666);
				if ret == -1 && std::io::Error::last_os_error().kind() != ErrorKind::AlreadyExists {
					crash_errno!("Failed to create file mountpoint at {:?}", target_path);
				}
			},
			run::Kind::Directory => {
				// Create a directory if it doesn't already exist.
				let ret = libc::mkdir(target_path.as_ptr(), 0o777);
				if ret == -1 && std::io::Error::last_os_error().kind() != ErrorKind::AlreadyExists {
					crash_errno!("Failed to create directory mountpoint at {:?}", target_path);
				}
			},
		}

		// For bind mounts, the filesystem type and mount options are NULL.
		let flags = libc::MS_BIND;
		let fs_type = std::ptr::null();
		let mount_options = std::ptr::null();

		// Perform the mount.
		let ret = libc::mount(
			source_path.as_ptr(),
			target_path.as_ptr(),
			fs_type,
			flags,
			mount_options,
		);
		if ret == -1 {
			crash_errno!(
				"Failed to bind-mount source {:?} to target {:?}",
				source_path,
				target_path
			);
		}

		// If the permission is read-only, remount as read-only.
		if mount.mode == run::Mode::ReadOnly {
			let ret = libc::mount(
				source_path.as_ptr(),
				target_path.as_ptr(),
				fs_type,
				flags | libc::MS_REMOUNT | libc::MS_RDONLY,
				mount_options,
			);
			if ret == -1 {
				crash_errno!("Failed to mount");
			}
		}
	}
}

unsafe fn create_etc_and_populate(ctx: &SandboxContext) {
	// Create `/etc`.
	{
		let mut path_buf = [0u8; libc::PATH_MAX as usize];
		let path = join_c!(&mut path_buf, ctx.host_chroot_path, "/etc");

		let ret = libc::mkdir(path.as_ptr(), 0o777);
		if ret == -1 {
			crash_errno!("Failed to create /etc");
		}
	}

	// Create `/etc/passwd`, describing the `tangram` user.
	let result = write_to_file::<1024>(
		format_args!("{}/etc/passwd", ctx.host_chroot_path),
		format_args!(
			concat!(
				"root:!:0:0:root:/nonexistent:/bin/false\n",
				"tangram:!:{}:{}:tangram:/home/tangram:/bin/false\n",
				"nobody:!:65534:65534:nobody:/nonexistent:/bin/false\n",
			),
			TANGRAM_UID, TANGRAM_GID
		),
	);
	if let Err(e) = result {
		crash!("Failed to write /etc/passwd: {}", e);
	}

	// Create `/etc/group`, describing the `tangram` group.
	let result = write_to_file::<1024>(
		format_args!("{}/etc/group", ctx.host_chroot_path),
		format_args!(concat!("tangram:x:{}:tangram\n",), TANGRAM_GID),
	);
	if let Err(e) = result {
		crash!("Failed to write /etc/group: {}", e);
	}

	// Create `/etc/nsswitch.conf`.
	let result = write_to_file::<1024>(
		format_args!("{}/etc/nsswitch.conf", ctx.host_chroot_path),
		concat!(
			"passwd:\tfiles compat\n",
			"shadow:\tfiles compat\n",
			"hosts:\tfiles dns compat\n",
		),
	);
	if let Err(e) = result {
		crash!("Failed to write /etc/nsswitch.conf: {}", e);
	}

	// If network access is enabled, copy /etc/resolv.conf.
	if ctx.network_enabled {
		let result = copy_file(
			"/etc/resolv.conf",
			format_args!("{}/etc/resolv.conf", ctx.host_chroot_path),
		);
		if let Err(e) = result {
			crash!("Failed to copy /etc/resolv.conf: {}", e);
		}
	}
}

unsafe fn mount_dev_proc_tmp(ctx: &SandboxContext) {
	// Mount /dev.
	{
		let source = b"/dev\0";

		let mut target_buf = [0u8; libc::PATH_MAX as usize];
		let target = join_c!(&mut target_buf, ctx.host_chroot_path, "/dev");

		let ret = libc::mkdir(target.as_ptr(), 0o777);
		if ret == -1 {
			crash_errno!("Failed to make mountpoint for /dev");
		}

		let ret = libc::mount(
			source.as_ptr().cast::<c_char>(),
			target.as_ptr(),
			std::ptr::null(),
			libc::MS_BIND | libc::MS_REC,
			std::ptr::null(),
		);
		if ret == -1 {
			crash_errno!("Failed to mount /dev");
		}
	}

	// Mount /proc.
	{
		let mut target_buf = [0u8; libc::PATH_MAX as usize];
		let target = join_c!(&mut target_buf, ctx.host_chroot_path, "/proc");

		let ret = libc::mkdir(target.as_ptr(), 0o777);
		if ret == -1 {
			crash_errno!("Failed to make mountpoint for /proc");
		}

		let ret = libc::mount(
			std::ptr::null(),
			target.as_ptr(),
			"proc\0".as_ptr().cast::<c_char>(),
			0,
			std::ptr::null(),
		);
		if ret == -1 {
			crash_errno!("Failed to mount /dev");
		}
	}

	// Mount /tmp.
	{
		let mut target_buf = [0u8; libc::PATH_MAX as usize];
		let target = join_c!(&mut target_buf, ctx.host_chroot_path, "/tmp");

		let ret = libc::mkdir(target.as_ptr(), 0o777);
		if ret == -1 {
			crash_errno!("Failed to make mountpoint for /tmp");
		}

		let ret = libc::mount(
			std::ptr::null(),
			target.as_ptr(),
			"tmpfs\0".as_ptr().cast::<c_char>(),
			0,
			std::ptr::null(),
		);
		if ret == -1 {
			crash_errno!("Failed to mount /tmp");
		}
	}
}

unsafe fn pivot_root_to_chroot_and_remove_old_root(ctx: &SandboxContext) {
	// The host's path to the new root directory.
	let new_root = &ctx.host_chroot_path;

	// Make sure the path ends in a trailing slash.
	let mut new_root_buf = [0u8; libc::PATH_MAX as usize];
	let new_root = join_c!(&mut new_root_buf, new_root, "/");

	// Make sure new_root is a mountpoint by bind-mounting it to itself.
	let ret = libc::mount(
		new_root.as_ptr(),
		new_root.as_ptr(),
		std::ptr::null(),
		libc::MS_BIND | libc::MS_REC | libc::MS_PRIVATE,
		std::ptr::null(),
	);
	if ret == -1 {
		crash_errno!("Failed to bind-mount new root {:?} to itself", new_root);
	}

	// Note: See pivot_root(2), section `pivot_root(".", ".")`, for details on why this approach is desirable. In short, this removes the need to create a temporary directory for the old root, and remove it after the pivot_root operation completes.

	// Change directory to the pivoted root.
	let ret = libc::chdir(new_root.as_ptr());
	if ret == -1 {
		crash_errno!("Failed to change directory to the new root");
	}

	let dot = b".\0";

	// Perform the pivot_root.
	let ret = libc::syscall(libc::SYS_pivot_root, dot.as_ptr(), dot.as_ptr());
	if ret == -1 {
		crash_errno!("Failed to pivot root");
	}

	// Remove the old mountpoint.
	let ret = libc::umount2(dot.as_ptr().cast::<c_char>(), libc::MNT_DETACH);
	if ret == -1 {
		crash_errno!("Failed to detach old mountpoint");
	}
}

/// Remount / as read-only. This leaves only the explicit read-write mountpoints as mutable inside the sandbox.
unsafe fn remount_root_as_read_only(_ctx: &SandboxContext) {
	let ret = libc::mount(
		std::ptr::null(),
		"/\0".as_ptr().cast::<c_char>(),
		std::ptr::null(),
		libc::MS_REMOUNT | libc::MS_BIND | libc::MS_REC | libc::MS_PRIVATE | libc::MS_RDONLY,
		std::ptr::null(),
	);
	if ret == -1 {
		crash_errno!("Failed to remount root as read-only");
	}
}

unsafe fn set_working_directory(ctx: &SandboxContext) {
	let ret = libc::chdir(ctx.working_directory.as_ptr());
	if ret == -1 {
		crash_errno!("Failed to set working directory");
	}
}
