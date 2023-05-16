use super::{
	mount::{self, Mount},
	server::Server,
	Process,
};
use crate::{
	error::{return_error, Error, Result, WrapErr},
	instance::Instance,
	operation, process,
	system::System,
	temp::Temp,
	util::fs,
};
use indoc::formatdoc;
use std::{
	collections::{BTreeMap, HashSet},
	ffi::{CStr, CString},
	os::{fd::IntoRawFd, unix::ffi::OsStrExt},
	sync::Arc,
};

/// The home directory guest path.
const HOME_DIRECTORY_GUEST_PATH: &str = "/home/tangram";

/// The socket guest path.
const SOCKET_GUEST_PATH: &str = "/socket";

/// The alignment of the stack to allocate for each process.
pub const STACK_ALIGN: usize = 16;

const STACK_LAYOUT: std::alloc::Layout =
	unsafe { std::alloc::Layout::from_size_align_unchecked(STACK_SIZE, STACK_ALIGN) };

/// The layout of the stack to allocate for each process.
pub const STACK_SIZE: usize = 2 << 21;

/// The UID for the tangram user.
pub const TANGRAM_UID: libc::uid_t = 1000;

/// The GID for the tangram user.
pub const TANGRAM_GID: libc::gid_t = 1000;

/// The working directory guest path.
const WORKING_DIRECTORY_GUEST_PATH: &str = "/home/tangram/work";

impl Process {
	#[allow(clippy::too_many_arguments)]
	pub async fn run_linux(
		tg: &Arc<Instance>,
		artifacts_guest_path: fs::PathBuf,
		system: System,
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

		// Add /usr/bin/env to the mounts.
		if !mounts.iter().any(|mount| {
			fs::Path::new("/usr/bin/env")
				.ancestors()
				.any(|path| mount.guest_path == path)
		}) {
			let env_host_path = match system {
				System::Amd64Linux => tg.assets_path().join("env_amd64_linux"),
				System::Arm64Linux => tg.assets_path().join("env_arm64_linux"),
				_ => unreachable!(),
			};
			mounts.insert(Mount {
				kind: mount::Kind::File,
				host_path: env_host_path,
				guest_path: "/usr/bin/env".into(),
				mode: mount::Mode::ReadOnly,
			});
		}

		// Add /bin/sh to the mounts.
		if !mounts.iter().any(|mount| {
			fs::Path::new("/bin/sh")
				.ancestors()
				.any(|path| mount.guest_path == path)
		}) {
			let sh_host_path = match system {
				System::Amd64Linux => tg.assets_path().join("sh_amd64_linux"),
				System::Arm64Linux => tg.assets_path().join("sh_arm64_linux"),
				_ => unreachable!(),
			};
			mounts.insert(Mount {
				kind: mount::Kind::File,
				host_path: sh_host_path,
				guest_path: "/bin/sh".into(),
				mode: mount::Mode::ReadOnly,
			});
		}

		// Add the artifacts directory to the mounts.
		mounts.insert(Mount {
			kind: mount::Kind::Directory,
			host_path: tg.artifacts_path(),
			guest_path: artifacts_guest_path.clone(),
			mode: mount::Mode::ReadOnly,
		});

		// Set `$HOME`.
		env.insert("HOME".to_owned(), HOME_DIRECTORY_GUEST_PATH.to_owned());

		// Create the socket path and set `$TANGRAM_SOCKET`.
		let socket_host_path = root_host_path.join("socket");
		env.insert(String::from("TANGRAM_SOCKET"), SOCKET_GUEST_PATH.to_owned());

		// Start the server.
		let server = Server::new(
			Arc::downgrade(tg),
			artifacts_guest_path,
			mounts.iter().cloned().collect(),
		);
		let server_task = tokio::spawn({
			let socket_host_path = socket_host_path.clone();
			async move {
				server.serve(&socket_host_path).await.unwrap();
			}
		});

		// Run the process.
		let status: ExitStatus = tokio::task::spawn_blocking(move || unsafe {
			run(
				&root_host_path,
				executable,
				env,
				args,
				mounts,
				network_enabled,
			)
		})
		.await
		.map_err(Error::other)
		.wrap_err("Failed to join the process task.")?
		.wrap_err("Failed to run the process.")?;

		// Stop the server.
		server_task.abort();
		server_task.await.ok();

		match status {
			ExitStatus::Code(0) => Ok(()),
			ExitStatus::Code(code) => Err(Error::Operation(operation::Error::Process(
				process::Error::Code(code),
			))),
			ExitStatus::Signal(signal) => Err(Error::Operation(operation::Error::Process(
				process::Error::Signal(signal),
			))),
		}
	}
}

#[allow(clippy::too_many_lines, clippy::similar_names)]
unsafe fn run(
	root_host_path: &fs::Path,
	executable: String,
	env: BTreeMap<String, String>,
	args: Vec<String>,
	mounts: HashSet<Mount, fnv::FnvBuildHasher>,
	network_enabled: bool,
) -> Result<ExitStatus> {
	// Create /etc.
	std::fs::create_dir_all(root_host_path.join("etc")).wrap_err("Failed to create /etc.")?;

	// Create /etc/passwd.
	std::fs::write(
		root_host_path.join("etc/passwd"),
		formatdoc!(
			r#"
				root:!:0:0:root:/nonexistent:/bin/false
				tangram:!:{TANGRAM_UID}:{TANGRAM_GID}:tangram:{HOME_DIRECTORY_GUEST_PATH}:/bin/false
				nobody:!:65534:65534:nobody:/nonexistent:/bin/false
			"#
		),
	)
	.wrap_err("Failed to create /etc/passwd.")?;

	// Create /etc/group.
	std::fs::write(
		root_host_path.join("etc/group"),
		formatdoc!(
			r#"
				tangram:x:{TANGRAM_GID}:tangram
			"#
		),
	)
	.wrap_err("Failed to create /etc/group.")?;

	// Create /etc/nsswitch.conf.
	std::fs::write(
		root_host_path.join("etc/nsswitch.conf"),
		formatdoc!(
			r#"
				passwd:	files compat
				shadow:	files compat
				hosts:	files dns compat
			"#
		),
	)
	.wrap_err("Failed to create /etc/nsswitch.conf.")?;

	// If network access is enabled, then copy /etc/resolv.conf from the host.
	if network_enabled {
		std::fs::copy("/etc/resolv.conf", root_host_path.join("etc/resolv.conf"))
			.wrap_err("Failed to copy /etc/resolv.conf.")?;
	}

	// Create the mount points.
	for mount in &mounts {
		// Create the target path.
		let target_path = root_host_path.join(mount.guest_path.strip_prefix("/").unwrap());

		// Create a mount point at the target path if one does not already exist.
		match mount.kind {
			mount::Kind::File => {
				std::fs::create_dir_all(target_path.parent().unwrap())
					.wrap_err("Failed to create the parent of the target path.")?;
				std::fs::write(&target_path, "").wrap_err_with(|| {
					let target_path = target_path.display();
					format!(r#"Failed to create a file for the target path "{target_path}"."#)
				})?;
			},

			mount::Kind::Directory => {
				std::fs::create_dir_all(&target_path).wrap_err_with(|| {
					let target_path = target_path.display();
					format!(r#"Failed to create a directory for the target path "{target_path}"."#)
				})?;
			},
		}
	}

	// Create the home directory.
	let home_directory_host_path =
		root_host_path.join(HOME_DIRECTORY_GUEST_PATH.strip_prefix('/').unwrap());
	std::fs::create_dir_all(&home_directory_host_path)
		.wrap_err("Failed to create the home directory.")?;

	// Create the working directory.
	let working_directory_host_path =
		root_host_path.join(WORKING_DIRECTORY_GUEST_PATH.strip_prefix('/').unwrap());
	std::fs::create_dir_all(&working_directory_host_path)
		.wrap_err("Failed to create the working directory.")?;

	// Create the stacks for the init and guest processes.
	let init_stack = Stack::new();
	let guest_stack = Stack::new();

	// Create the socket.
	let (host_socket, guest_socket) = std::os::unix::net::UnixStream::pair()
		.map_err(Error::other)
		.wrap_err("Failed to create the socket pair.")?;
	let host_socket_fd = host_socket.into_raw_fd();
	let guest_socket_fd = guest_socket.into_raw_fd();

	// Get the root host path.
	let root_host_path = CString::new(root_host_path.as_os_str().as_bytes())
		.map_err(Error::other)
		.wrap_err("The root host path is not a valid C string.")?;

	// Get the working directory guest path.
	let working_directory_guest_path = CString::new(WORKING_DIRECTORY_GUEST_PATH)
		.map_err(Error::other)
		.wrap_err("Working directory cannot be made into a C string")?;

	// Get the executable.
	let executable = CString::new(executable)
		.map_err(Error::other)
		.wrap_err("The executable is not a valid C string.")?;

	// Create `envp`.
	let env = env
		.into_iter()
		.map(|(k, v)| format!("{k}={v}"))
		.map(|a| CString::new(a).unwrap())
		.collect::<Vec<_>>();
	let mut envp = Vec::with_capacity(env.len() + 1);
	for pair in &env {
		envp.push(pair.as_ptr());
	}
	envp.push(std::ptr::null());

	// Create `argv`.
	let args = args
		.into_iter()
		.map(|a| {
			CString::new(a)
				.map_err(Error::other)
				.wrap_err("Argument could not be made into a C string.")
		})
		.collect::<Result<Vec<_>>>()?;
	let mut argv: Vec<*const libc::c_char> = Vec::with_capacity(1 + args.len() + 1);
	argv.push(executable.as_ptr());
	for arg in &args {
		argv.push(arg.as_ptr());
	}
	argv.push(std::ptr::null());

	// Get the file descriptor to redirect the guest process's stdout and stderr to.
	let log_fd = libc::STDERR_FILENO;

	// Collect the mounts.
	let mounts = mounts.into_iter().collect::<Vec<_>>();

	// Create the context.
	let mut context = Context {
		init_stack: init_stack.as_mut_ptr(),
		guest_stack: guest_stack.as_mut_ptr(),
		log_fd,
		host_socket_fd,
		guest_socket_fd,
		root_host_path: root_host_path.as_ptr(),
		working_directory_guest_path: working_directory_guest_path.as_ptr(),
		executable: executable.as_ptr(),
		argv: argv.as_ptr(),
		envp: envp.as_ptr(),
		mounts: &mounts,
		network_enabled,
	};

	// Spawn the init process.
	let init_process_pid = libc::clone(
		init,
		context.init_stack.cast(),
		libc::CLONE_NEWUSER,
		std::ptr::addr_of_mut!(context).cast(),
	);
	if init_process_pid == -1 {
		return Err(Error::last_os_error().wrap("Failed to clone the init process."));
	}

	// Receive the guest process's PID from the socket.
	let Some(guest_process_pid) = socket_recv::<libc::pid_t>(host_socket_fd) else {
		return Err(Error::last_os_error()
			.wrap("Failed to receive the PID of the guest process from the socket."));
	};

	// Write the guest process's UID map.
	std::fs::write(
		format!("/proc/{guest_process_pid}/uid_map"),
		format!("{TANGRAM_UID} {} 1\n", libc::getuid()),
	)
	.wrap_err("Failed to set the UID map.")?;

	// Deny setgroups to the process.
	std::fs::write(format!("/proc/{guest_process_pid}/setgroups"), "deny")
		.wrap_err("Failed to disable setgroups.")?;

	// Write the guest process's GID map.
	std::fs::write(
		format!("/proc/{guest_process_pid}/gid_map"),
		format!("{TANGRAM_GID} {} 1\n", libc::getgid()),
	)
	.wrap_err("Failed to set the GID map.")?;

	// Notify the guest process that it can continue.
	let ret = socket_send(host_socket_fd, &1u8);
	if ret == -1 {
		return Err(
			Error::last_os_error().wrap("Failed to notify the guest process that it can continue.")
		);
	}

	// Receive the exit status of the guest process from the init process.
	let Some(guest_process_exit_status) = socket_recv::<ExitStatus>(host_socket_fd) else {
		return Err(Error::last_os_error().wrap("Failed to receive the exit status from the init process."));
	};

	// Wait for the init process.
	let mut status: libc::c_int = 0;
	let ret = unsafe { libc::waitpid(init_process_pid, &mut status, libc::__WALL) };
	if ret == -1 {
		return Err(Error::last_os_error().wrap("Failed to wait for the init process."));
	}
	let init_process_exit_status = if libc::WIFEXITED(status) {
		let status = libc::WEXITSTATUS(status);
		ExitStatus::Code(status)
	} else if libc::WIFSIGNALED(status) {
		let signal = libc::WTERMSIG(status);
		ExitStatus::Signal(signal)
	} else {
		unreachable!();
	};
	if init_process_exit_status != ExitStatus::Code(0) {
		return_error!("The init process did not exit successfully.");
	}

	Ok(guest_process_exit_status)
}

pub extern "C" fn init(arg: *mut libc::c_void) -> libc::c_int {
	unsafe {
		let context = &mut *arg.cast::<Context<'_>>();
		init_inner(context)
	}
}

unsafe fn init_inner(context: &mut Context) -> i32 {
	// Ask to be SIGKILL'd if the host process exits.
	let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
	if ret == -1 {
		abort_errno!("Failed to set PDEATHSIG.");
	}

	// Duplicate stdout and stderr to the log.
	let ret = libc::dup2(context.log_fd, libc::STDOUT_FILENO);
	if ret == -1 {
		abort_errno!("Failed to duplicate stdout to the log.");
	}
	let ret = libc::dup2(context.log_fd, libc::STDERR_FILENO);
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
	let guest_process_pid: libc::pid_t = libc::clone(
		guest,
		context.init_stack.cast(),
		libc::CLONE_NEWNS | libc::CLONE_NEWPID | network_clone_flags,
		(context as *mut Context).cast(),
	);
	if guest_process_pid == -1 {
		abort_errno!("Failed to spawn the guest process.");
	}

	// Send the guest process's PID to the parent, so the parent can write the UID and GID maps.
	let ret = socket_send(context.guest_socket_fd, &guest_process_pid);
	if ret == -1 {
		abort_errno!("Failed to send the PID of guest process.");
	};

	// Wait for the guest process.
	let mut status: libc::c_int = 0;
	let ret = unsafe { libc::waitpid(guest_process_pid, &mut status, libc::__WALL) };
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

	// Send the parent the exit code of the guest process.
	let ret = socket_send(context.guest_socket_fd, &guest_process_exit_status);
	if ret == -1 {
		abort_errno!("Failed to send the guest process's exit status back to the host.");
	};

	0
}

pub extern "C" fn guest(arg: *mut libc::c_void) -> libc::c_int {
	unsafe {
		let context = &mut *arg.cast::<Context<'_>>();
		guest_inner(context)
	}
}

#[allow(clippy::too_many_lines)]
unsafe fn guest_inner(context: &mut Context) -> i32 {
	// Ask to receive a SIGKILL signal if the host process exits.
	let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
	if ret == -1 {
		abort_errno!("Failed to set PDEATHSIG.");
	}

	// Wait for the notification from the host process to continue.
	let Some(notification) = socket_recv::<u8>(context.guest_socket_fd) else {
		abort_errno!("The guest process failed to receive the notification from the host process to continue.");
	};
	assert_eq!(notification, 1);

	// Mount /dev.
	let source = b"/dev\0";
	let mut target_buf = [0u8; libc::PATH_MAX as usize];
	let target = write_buf!(
		&mut target_buf,
		"{}/dev",
		CStr::from_ptr(context.root_host_path).to_str().unwrap()
	);
	let ret = libc::mkdir(target.as_ptr(), 0o777);
	if ret == -1 {
		abort_errno!("Failed to create /dev.");
	}
	let ret = libc::mount(
		source.as_ptr().cast(),
		target.as_ptr(),
		std::ptr::null(),
		libc::MS_BIND | libc::MS_REC,
		std::ptr::null(),
	);
	if ret == -1 {
		abort_errno!("Failed to mount /dev.");
	}

	// Mount /proc.
	let mut target_buf = [0u8; libc::PATH_MAX as usize];
	let target = write_buf!(
		&mut target_buf,
		"{}/proc",
		CStr::from_ptr(context.root_host_path).to_str().unwrap(),
	);
	let ret = libc::mkdir(target.as_ptr(), 0o777);
	if ret == -1 {
		abort_errno!("Failed to create /proc.");
	}
	let ret = libc::mount(
		std::ptr::null(),
		target.as_ptr(),
		"proc\0".as_ptr().cast(),
		0,
		std::ptr::null(),
	);
	if ret == -1 {
		abort_errno!("Failed to mount /dev.");
	}

	// Mount /tmp.
	let mut target_buf = [0u8; libc::PATH_MAX as usize];
	let target = write_buf!(
		&mut target_buf,
		"{}/tmp",
		CStr::from_ptr(context.root_host_path).to_str().unwrap(),
	);
	let ret = libc::mkdir(target.as_ptr(), 0o777);
	if ret == -1 {
		abort_errno!("Failed to create /tmp.");
	}
	let ret = libc::mount(
		std::ptr::null(),
		target.as_ptr(),
		"tmpfs\0".as_ptr().cast(),
		0,
		std::ptr::null(),
	);
	if ret == -1 {
		abort_errno!("Failed to mount /tmp.");
	}

	// Perform the mounts.
	for mount in context.mounts {
		// Create the source path.
		let mut source_path_buf = [0u8; libc::PATH_MAX as usize];
		let source_path = write_buf!(
			&mut source_path_buf,
			"{}",
			mount.host_path.to_str().unwrap(),
		);

		// Create the target path.
		let mut target_path_buf = [0u8; libc::PATH_MAX as usize];
		let target_path = write_buf!(
			&mut target_path_buf,
			"{}{}",
			CStr::from_ptr(context.root_host_path).to_str().unwrap(),
			mount.guest_path.to_str().unwrap(),
		);

		// Perform the mount.
		let fs_type = std::ptr::null();
		let flags = libc::MS_BIND;
		let mount_options = std::ptr::null();
		let ret = libc::mount(
			source_path.as_ptr(),
			target_path.as_ptr(),
			fs_type,
			flags,
			mount_options,
		);
		if ret == -1 {
			abort_errno!(
				r#"Failed to mount "{}" to "{}"."#,
				source_path.to_str().unwrap(),
				target_path.to_str().unwrap(),
			);
		}

		// If the mode is read-only, then remount the path as read-only.
		if mount.mode == mount::Mode::ReadOnly {
			let ret = libc::mount(
				source_path.as_ptr(),
				target_path.as_ptr(),
				fs_type,
				flags | libc::MS_REMOUNT | libc::MS_RDONLY,
				mount_options,
			);
			if ret == -1 {
				abort_errno!("Failed to remount the path as read-only.");
			}
		}
	}

	// Mount the root.
	let ret = libc::mount(
		context.root_host_path,
		context.root_host_path,
		std::ptr::null(),
		libc::MS_BIND | libc::MS_REC | libc::MS_PRIVATE,
		std::ptr::null(),
	);
	if ret == -1 {
		abort_errno!("Failed to mount the root.");
	}

	// Change the working directory to the pivoted root.
	let ret = libc::chdir(context.root_host_path);
	if ret == -1 {
		abort_errno!("Failed to change directory to the root.");
	}

	// Pivot the root.
	let dot = b".\0";
	let ret = libc::syscall(libc::SYS_pivot_root, dot.as_ptr(), dot.as_ptr());
	if ret == -1 {
		abort_errno!("Failed to pivot the root.");
	}

	// Unmount the root.
	let ret = libc::umount2(dot.as_ptr().cast(), libc::MNT_DETACH);
	if ret == -1 {
		abort_errno!("Failed to unmount the root.");
	}

	// Remount the root as read-only.
	let ret = libc::mount(
		std::ptr::null(),
		"/\0".as_ptr().cast(),
		std::ptr::null(),
		libc::MS_REMOUNT | libc::MS_BIND | libc::MS_REC | libc::MS_PRIVATE | libc::MS_RDONLY,
		std::ptr::null(),
	);
	if ret == -1 {
		abort_errno!("Failed to remount the root as read-only.");
	}

	// Set the working directory.
	let ret = libc::chdir(context.working_directory_guest_path);
	if ret == -1 {
		abort_errno!("Failed to set the working directory.");
	}

	// Exec.
	libc::execve(context.executable, context.argv, context.envp);
	abort_errno!(r#"Failed to call execve."#);
}

/// Shared context between the host, init, guest, and guest processes.
pub struct Context<'a> {
	/// The init process's stack.
	pub init_stack: *mut u8,

	/// The guest process's stack.
	pub guest_stack: *mut u8,

	/// The file descriptor of the host side of the socket.
	pub host_socket_fd: libc::c_int,

	/// The file descriptor of the guest side of the socket.
	pub guest_socket_fd: libc::c_int,

	/// The file descriptor to write the guest process's stdout and stderr to.
	pub log_fd: libc::c_int,

	/// The host path to the root.
	pub root_host_path: *const libc::c_char,

	/// The guest path to the working directory.
	pub working_directory_guest_path: *const libc::c_char,

	/// The name of the executable to run.
	pub executable: *const libc::c_char,

	/// The command line arguments to pass to the guest process. Each argument must be a `NULL`-terminated string. The last element of `argv` must be `NULL`.
	pub argv: *const *const libc::c_char,

	/// The environment variables to pass to the guest process. Each environment variable must be in the form `KEY=VALUE`, terminated by `NULL`. The last element of `envp` must be `NULL`.
	pub envp: *const *const libc::c_char,

	/// The paths to mount in the guest process.
	pub mounts: &'a [Mount],

	/// Whether to enable network access for the guest process.
	pub network_enabled: bool,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ExitStatus {
	Code(i32),
	Signal(i32),
}

pub struct Stack(*mut u8);

impl Stack {
	/// Create a new stack.
	pub fn new() -> Self {
		let pointer = unsafe { std::alloc::alloc(STACK_LAYOUT) };
		Self(pointer)
	}

	/// Get a pointer to the top of the stack.
	pub fn as_mut_ptr(&self) -> *mut u8 {
		unsafe { self.0.add(STACK_SIZE) }
	}
}

impl Drop for Stack {
	fn drop(&mut self) {
		unsafe { std::alloc::dealloc(self.0, STACK_LAYOUT) };
	}
}

pub unsafe fn socket_send<T: Copy>(sock_fd: libc::c_int, data: &T) -> i32 {
	let ret = libc::send(
		sock_fd,
		(data as *const T).cast(),
		std::mem::size_of_val(data),
		0,
	);
	if ret == -1 {
		return -1;
	}
	0
}

pub unsafe fn socket_recv<T: Copy>(sock_fd: libc::c_int) -> Option<T> {
	let mut data = std::mem::MaybeUninit::<T>::uninit();
	let ret = libc::recv(
		sock_fd,
		data.as_mut_ptr().cast(),
		std::mem::size_of::<T>(),
		0,
	);
	if ret == -1 {
		return None;
	}
	Some(data.assume_init())
}

macro_rules! write_buf {
	($buf:expr, $($t:tt)*) => {{
		use ::std::io::Write;
		let buf: &mut [u8] = &mut *$buf;
		let mut cursor = ::std::io::Cursor::new(buf);
		::std::write!(cursor, $($t)*).unwrap();
		let last_index = usize::try_from(cursor.position()).unwrap();
		let buf = cursor.into_inner();
		::std::ffi::CStr::from_bytes_with_nul(&buf[0..last_index + 1]).unwrap()
	}}
}
pub(crate) use write_buf;

macro_rules! abort {
	($($t:tt)*) => {{
		eprintln!("Error: {}", format_args!($($t)*));
		std::process::exit(1)
	}};
}
pub(crate) use abort;

macro_rules! abort_errno {
	($($t:tt)*) => {{
		eprintln!("Error: {}", format_args!($($t)*));
		eprintln!("\t{}", std::io::Error::last_os_error());
		std::process::exit(1)
	}};
}
pub(crate) use abort_errno;
