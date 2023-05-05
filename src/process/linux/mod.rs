use super::{run, server::Server, Process};
use crate::{
	error::{return_error, Error, Result, WrapErr},
	instance::Instance,
	system::System,
	temp::Temp,
	util::fs,
};
use libc::{c_char, c_int, c_void};
use std::{
	collections::{BTreeMap, HashSet},
	ffi::{CStr, CString, OsStr},
	io::Write,
	mem::MaybeUninit,
	os::{fd::IntoRawFd, unix::ffi::OsStrExt},
	sync::Arc,
};

mod inner;
mod outer;

/// The UID for the tangram user in the sandbox.
pub const TANGRAM_UID: libc::uid_t = 1000;

/// The GID for the tangram user in the sandbox.
pub const TANGRAM_GID: libc::gid_t = 1000;

/// The size of the stack to allocate for each child process.
const STACK_SIZE: usize = 1024 * 1000 * 8;

impl Process {
	pub async fn run_linux(
		tg: &Arc<Instance>,
		_system: System,
		command: String,
		mut env: BTreeMap<String, String>,
		args: Vec<String>,
		mut paths: HashSet<run::Path, fnv::FnvBuildHasher>,
		network_enabled: bool,
	) -> Result<()> {
		// Create a temp path for the root directory.
		let root_directory = Temp::new(tg);

		// Add the home directory to the root directory and set the HOME environment variable if it is unset.
		let home_directory_path = root_directory.path().join("home").join("tangram");
		tokio::fs::create_dir_all(&home_directory_path)
			.await
			.wrap_err("Failed to create the home directory.")?;
		env.entry(String::from("HOME"))
			.or_insert_with(|| "/home/tangram".to_owned());

		// Add the working directory to the home directory.
		let working_directory_path = home_directory_path.join("work");
		let working_directory_guest_path = fs::PathBuf::from("/home/tangram/work");
		tokio::fs::create_dir_all(&working_directory_path)
			.await
			.wrap_err("Failed to create the working directory.")?;

		// Mount the home directory as read-write
		let home_directory_guest_path = fs::PathBuf::from("/home/tangram");
		paths.insert(run::Path {
			kind: run::Kind::Directory,
			host_path: home_directory_path,
			guest_path: home_directory_guest_path.clone(),
			mode: run::Mode::ReadWrite,
		});

		// Get the right name of the /bin/sh to use.
		#[cfg(target_arch = "x86_64")]
		let sh_name = "sh_amd64_linux";
		#[cfg(target_arch = "aarch64")]
		let sh_name = "sh_arm64_linux";

		// Mount /bin/sh from ~/.tangram/assets
		paths.insert(run::Path {
			kind: run::Kind::File,
			host_path: tg.path().join("assets").join(sh_name),
			guest_path: "/bin/sh".into(),
			mode: run::Mode::ReadOnly,
		});

		// Get the right name of the /usr/bin/env to use.
		#[cfg(target_arch = "x86_64")]
		let env_name = "env_amd64_linux";
		#[cfg(target_arch = "aarch64")]
		let env_name = "env_arm64_linux";

		// Mount /bin/sh from ~/.tangram/assets
		paths.insert(run::Path {
			kind: run::Kind::File,
			host_path: tg.path().join("assets").join(env_name),
			guest_path: "/usr/bin/env".into(),
			mode: run::Mode::ReadOnly,
		});

		// Create the socket path, and set the TANGRAM_SOCKET environment variable.
		let socket_path = root_directory.path().join("socket");
		env.insert(String::from("TANGRAM_SOCKET"), "/socket".to_owned());

		// Start the server.
		let server = Server::new(Arc::downgrade(tg), paths.iter().cloned().collect());
		let server_task = tokio::spawn({
			let socket_path = socket_path.clone();
			async move {
				server.serve(&socket_path).await.unwrap();
			}
		});

		// Check to make sure all the mount sources actually exist. We can't mount files or directories that don't exist.
		// NOTE: If `path` is a symlink, mount(2) will follow it---so we're really checking if the destination exists here.
		for path in &paths {
			if !path.host_path.exists() {
				return_error!(
					"Host path {:?} (or symlink target) does not exist; cannot mount.",
					path.host_path
				);
			}
		}

		// Get the root directory path.
		let root_directory_path = root_directory.path().to_owned();

		// Prepare the sandbox context, and execute the process in the sandbox.
		let exit: ExitStatus = tokio::task::spawn_blocking(move || unsafe {
			prepare_context_and_exec_child(
				root_directory_path,
				working_directory_guest_path,
				command,
				env,
				args,
				paths,
				network_enabled,
			)
		})
		.await
		.map_err(Error::other)
		.wrap_err("Failed to join task responsible for spawning subprocess.")??;

		// Stop the server.
		server_task.abort();
		server_task.await.ok();

		if let ExitStatus::ExitCode(0) = exit {
			Ok(())
		} else {
			match exit {
				ExitStatus::ExitCode(code) => return_error!("Process exited with code {}", code),
				ExitStatus::Signal(signal) => {
					return_error!("Process exited with signal {}", signal)
				},
			}
		}
	}
}

struct SandboxContext<'a> {
	/// A pointer to the outer child's stack.
	pub outer_child_stack: &'a mut [u8],

	/// A pointer to the inner child's stack.
	pub inner_child_stack: &'a mut [u8],

	/// The socket used to send the outer-namespace pid of the inner process, and wait for a confirmation that the uid map and gid map have been set.
	pub coordination_socket_outer_fd: c_int,
	pub coordination_socket_inner_fd: c_int,

	/// File descriptor into which the stdout and stderr of the child process will be redirected.
	pub stdio_fd: c_int,

	/// Path to the guest chroot, as a directory on the host.
	pub host_chroot_path: &'a str,

	/// Enable network access for the guest process.
	pub network_enabled: bool,

	/// List of mounts to expose to the guest.
	pub mounts: &'a [run::Path],

	/// Name of the executable to run. This will be resolved on `$PATH` if it does not contain a `/` character.
	pub executable: &'a CStr,

	/// Path to the working directory inside the guest.
	pub working_directory: &'a CStr,

	/// Arguments to pass to the subprocess. Each argument must be a `NULL`-terminated string. The last element of `argv` must be `NULL`.
	pub argv: &'a [*const c_char],

	/// Environment variables to pass to the process. Each environment variable must be in the form `KEY=VALUE`, terminated by `NULL`. The last element of `envp` must be `NULL`.
	pub envp: &'a [*const c_char],
}

/// Prepare the SandboxContext, and exec.
unsafe fn prepare_context_and_exec_child(
	chroot_path: fs::PathBuf,
	working_directory: fs::PathBuf,
	executable: String,
	env: BTreeMap<String, String>,
	args: Vec<String>,
	paths: HashSet<run::Path, fnv::FnvBuildHasher>,
	network_enabled: bool,
) -> Result<ExitStatus> {
	// Create the coordination socket.
	let (coordination_outer, coordination_inner) = std::os::unix::net::UnixStream::pair()
		.map_err(Error::other)
		.wrap_err("Failed to create the coordination socket pair.")?;

	// Get the path to the chroot.
	let chroot_path = chroot_path
		.to_str()
		.wrap_err("The path to the chroot was not valid UTF-8.")?;

	// Get the executable name.
	let executable = CString::new(executable)
		.map_err(Error::other)
		.wrap_err("The executable name is not a valid C string.")?;

	// Get the working directory path.
	let working_directory = CString::new(working_directory.as_os_str().as_bytes())
		.map_err(Error::other)
		.wrap_err("Working directory cannot be made into a C string")?;

	// Get the args.
	let args = args
		.into_iter()
		.map(|a| {
			CString::new(a)
				.map_err(Error::other)
				.wrap_err("Argument could not be made into a C string.")
		})
		.collect::<Result<Vec<_>>>()?;

	// Get the environment variables.
	let env = env
		.into_iter()
		.map(|(k, v)| format!("{k}={v}"))
		.map(|a| CString::new(a).unwrap())
		.collect::<Vec<_>>();

	// Construct `argv`, pointing into `args`, starting with `executable`, and with a nul to terminate the array.
	let mut argv: Vec<*const c_char> = Vec::with_capacity(args.len() + 2);
	argv.push(executable.as_ptr());
	for arg in &args {
		argv.push(arg.as_ptr());
	}
	argv.push(std::ptr::null());

	// Construct `envp`, pointing into `env`, with a nul to terminate the array.
	let mut envp = Vec::with_capacity(env.len() + 1);
	for pair in &env {
		envp.push(pair.as_ptr());
	}
	envp.push(std::ptr::null());

	// Allocate stacks for the outer and inner child processes.
	let mut outer_child_stack = vec![0; STACK_SIZE];
	let mut inner_child_stack = vec![0; STACK_SIZE];

	// Get the file descriptor to send child process output into.
	let stdio_fd = libc::STDERR_FILENO;

	// Collect the paths into a Vec.
	let paths = paths.into_iter().collect::<Vec<_>>();

	// Create the sandbox context.
	let mut ctx = SandboxContext {
		outer_child_stack: &mut outer_child_stack,
		inner_child_stack: &mut inner_child_stack,
		stdio_fd,
		coordination_socket_outer_fd: coordination_outer.into_raw_fd(),
		coordination_socket_inner_fd: coordination_inner.into_raw_fd(),
		host_chroot_path: chroot_path,
		mounts: &paths,
		working_directory: &working_directory,
		network_enabled,
		executable: executable.as_c_str(),
		argv: argv.as_slice(),
		envp: envp.as_slice(),
	};

	// Spawn the outer child with clone, unsharing its user namespace.
	let outer_child_pid = libc::clone(
		outer::outer_child_callback,
		top_stack_addr(&mut ctx.outer_child_stack),
		libc::CLONE_NEWUSER,
		&mut ctx as *mut _ as *mut c_void,
	);
	if outer_child_pid == -1 {
		return Err(Error::last_os_error().wrap("Failed to clone outer child."));
	}

	// Receive the inner child's pid from the coordination socket.
	let Ok(inner_child_pid) = socket_recv::<libc::pid_t>(ctx.coordination_socket_outer_fd) else {
		return_error!("Failed to receive pid of inner child from coordination socket");
	};

	// Write the inner child's UID map.
	write_to_file::<256>(
		format_args!("/proc/{inner_child_pid}/uid_map"),
		format_args!("{TANGRAM_UID} {} 1\n", libc::getuid()),
	)
	.wrap_err("Failed to set UID map.")?;

	// Deny setgroups to the child.
	write_to_file::<32>(format_args!("/proc/{inner_child_pid}/setgroups"), "deny")
		.wrap_err("Failed to disable setgroups.")?;

	// Write the inner child's GID map.
	write_to_file::<256>(
		format_args!("/proc/{inner_child_pid}/gid_map"),
		format_args!("{TANGRAM_GID} {} 1\n", libc::getgid()),
	)
	.wrap_err("Failed to set gid map.")?;

	// Send the inner child the OK to continue.
	socket_send(ctx.coordination_socket_outer_fd, &1u8)
		.wrap_err("Failed to send child OK to continue.")?;

	// Receive the exit status of the inner child from the outer child.
	let inner_child_exit = socket_recv::<ExitStatus>(ctx.coordination_socket_outer_fd)
		.wrap_err("Failed to receive exit status from outer child.")?;

	// Reap the outer child.
	let outer_child_exit = waitpid(outer_child_pid).wrap_err("Failed to reap outer child.")?;
	if outer_child_exit != ExitStatus::ExitCode(0) {
		return_error!(
			"Outer child must exit successfully, instead got {:?}.",
			outer_child_exit
		);
	}

	Ok(inner_child_exit)
}

macro_rules! crash {
	($msg:literal $(,)?) => {{
		eprintln!("Error: {}", $msg);
		std::process::exit(1);
	}};
	($err:expr $(,)?) => {{
		eprintln!("Error: {}", $err);
		std::process::exit(1);
	}};
	($fmt:expr, $($arg:expr)*) => {{
		eprint!("Error: ");
		eprintln!($fmt, $($arg)*);
		std::process::exit(1)
	}};
}
use crash;

macro_rules! crash_errno {
	($msg:literal $(,)?) => {{
		let os_err = std::io::Error::last_os_error();
		eprintln!("Error: {}: {os_err}", $msg);
		std::process::exit(1);
	}};
	($err:expr $(,)?) => {{
		let os_err = std::io::Error::last_os_error();
		eprintln!("Error: {}: {os_err}", $err);
		std::process::exit(1);
	}};
	($fmt:expr, $($arg:expr),*) => {{
		eprint!("Error: ");
		eprint!($fmt, $($arg),*);
		eprintln!(": {}", std::io::Error::last_os_error());
		std::process::exit(1)
	}};
}
use crash_errno;

/// Get the top address in a stack, aligned correctly, to pass to clone(2).
unsafe fn top_stack_addr(data: &mut [u8]) -> *mut c_void {
	let top_ptr = data.as_mut_ptr_range().end;
	// Align the top pointer on a 16-byte boundary.
	top_ptr.sub(top_ptr as usize % 16).cast::<libc::c_void>()
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ExitStatus {
	ExitCode(i32),
	Signal(i32),
}

fn waitpid(pid: libc::pid_t) -> std::io::Result<ExitStatus> {
	let mut wait_status: libc::c_int = 0;
	if unsafe { libc::waitpid(pid, &mut wait_status as *mut _, libc::__WALL) } == -1 {
		return Err(std::io::Error::last_os_error());
	}

	if libc::WIFEXITED(wait_status) {
		let status = libc::WEXITSTATUS(wait_status);
		Ok(ExitStatus::ExitCode(status))
	} else if libc::WIFSIGNALED(wait_status) {
		let signal = libc::WTERMSIG(wait_status);
		Ok(ExitStatus::Signal(signal))
	} else {
		panic!(r#"Process "{pid}" exited without status or signal."#);
	}
}

unsafe fn socket_send<T: Copy>(sock_fd: libc::c_int, data: &T) -> std::io::Result<()> {
	let return_code = unsafe {
		libc::send(
			sock_fd,
			data as *const _ as *const c_void,
			std::mem::size_of::<T>(),
			0,
		)
	};
	if return_code == -1 {
		Err(std::io::Error::last_os_error())
	} else {
		Ok(())
	}
}

unsafe fn socket_recv<T: Copy>(sock_fd: libc::c_int) -> std::io::Result<T> {
	let mut data = std::mem::MaybeUninit::<T>::uninit();
	let return_code = unsafe {
		libc::recv(
			sock_fd,
			data.as_mut_ptr() as *mut _ as *mut c_void,
			std::mem::size_of::<T>(),
			0,
		)
	};
	if return_code == -1 {
		Err(std::io::Error::last_os_error())
	} else {
		Ok(data.assume_init())
	}
}

macro_rules! write_c {
	($buf:expr, $fmt:expr, $($arg:tt)*) => {
		$crate::process::linux::__write_c_inner($buf, format_args!($fmt, $($arg)*))
	}
}
pub(crate) use write_c;

fn __write_c_inner<'a>(buf: &'a mut [u8], args: std::fmt::Arguments) -> &'a CStr {
	// Write to the buffer with a cursor.
	let mut cursor = ::std::io::Cursor::new(&mut *buf);
	write!(cursor, "{args}\0").expect("failed to write format string to fixed-size buffer");
	let last_index: usize = cursor.position() as usize;
	drop(cursor);

	// Cast the data to a CStr.
	CStr::from_bytes_with_nul(&buf[0..last_index]).unwrap()
}

macro_rules! join_c {
	($buf:expr, $($arg:expr),*) => {{
		use ::std::io::Write;

		// Write to the buffer with a cursor.
		let buf: &mut [u8] = &mut *$buf;
		let mut cursor = ::std::io::Cursor::new(buf);
		$(
				::std::io::Write::write_all(&mut cursor, $arg.as_ref()).expect("failed to write joined part to buffer");
		)*
		write!(cursor, "\0").expect("failed to write trailing NUL byte to buffer");

		let last_index: usize = cursor.position() as usize;
		let buf = cursor.into_inner();

		// Cast the data to a CStr.
		CStr::from_bytes_with_nul(&buf[0..last_index]).unwrap()
	}}
}
pub(crate) use join_c;

/// Open or create the file at `path` and write `contents` into it.
fn write_to_file<const BUF_LEN: usize>(
	path: impl std::fmt::Display,
	contents: impl std::fmt::Display,
) -> std::io::Result<()> {
	// Format the pathname.
	let mut path_buf = [0u8; libc::PATH_MAX as usize];
	let path_c = write_c!(&mut path_buf, "{}", path);

	// Open the file.
	let fd = unsafe {
		libc::open(
			path_c.as_ptr(),
			libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
			0o644,
		)
	};
	if fd == -1 {
		return Err(std::io::Error::last_os_error());
	}

	// Format the contents.
	let mut contents_buf = [0u8; BUF_LEN];
	let contents_c = write_c!(&mut contents_buf, "{}", contents);

	// Write the bytes to the file.
	let result = unsafe { write_all(fd, contents_c.to_bytes()) };

	// Close the file.
	if unsafe { libc::close(fd) } == -1 {
		return result.and(Err(std::io::Error::last_os_error()));
	}

	result
}

/// Copy a file from `source` to `dest`.
fn copy_file(source: impl std::fmt::Display, dest: impl std::fmt::Display) -> std::io::Result<()> {
	// Open the source file.
	let mut source_buf = [0u8; libc::PATH_MAX as usize];
	let source_c = write_c!(&mut source_buf, "{}", source);
	let source_fd = unsafe { libc::open(source_c.as_ptr(), libc::O_RDONLY) };
	if source_fd == -1 {
		return Err(std::io::Error::last_os_error());
	}

	// Open the destination file.
	let mut dest_buf = [0u8; libc::PATH_MAX as usize];
	let dest_c = write_c!(&mut dest_buf, "{}", dest);
	let dest_fd = unsafe {
		libc::open(
			dest_c.as_ptr(),
			libc::O_CREAT | libc::O_WRONLY | libc::O_TRUNC,
			0o644,
		)
	};
	if dest_fd == -1 {
		return Err(std::io::Error::last_os_error());
	}

	// Get the length of the source file.
	let mut source_stat: MaybeUninit<libc::stat> = MaybeUninit::uninit();
	let ret = unsafe { libc::fstat(source_fd, source_stat.as_mut_ptr()) };
	if ret == -1 {
		return Err(std::io::Error::last_os_error());
	}
	let source_stat = unsafe { source_stat.assume_init() };

	// Copy between the two files with sendfile.
	let mut len = source_stat.st_size as usize;
	let copy_result = loop {
		let ret = unsafe { libc::sendfile(dest_fd, source_fd, std::ptr::null_mut(), len) };
		if ret == -1 {
			break Err(std::io::Error::last_os_error());
		}

		len -= ret as usize;
		if len == 0 {
			break Ok(());
		}
	};

	// Close the source file.
	let ret = unsafe { libc::close(source_fd) };
	if ret == -1 {
		let _ = unsafe { libc::close(dest_fd) };
		return Err(std::io::Error::last_os_error());
	}

	// Close the destination file.
	let ret = unsafe { libc::close(dest_fd) };
	if ret == -1 {
		return Err(std::io::Error::last_os_error());
	}

	// Return the result of the copy.
	copy_result
}

/// Write `buf` to `fd`.
unsafe fn write_all(fd: c_int, buf: &[u8]) -> std::io::Result<()> {
	let mut bytes_written: usize = 0;
	while bytes_written < buf.len() {
		let slice = &buf[bytes_written..];
		let ret = libc::write(fd, slice.as_ptr().cast::<libc::c_void>(), slice.len());
		if ret == -1 {
			return Err(std::io::Error::last_os_error());
		}
		bytes_written += ret as usize;
	}
	Ok(())
}

/// Recursively create a directory and all of its ancestors.
unsafe fn mkdir_p(path: &dyn AsRef<[u8]>) -> std::io::Result<()> {
	let path_os = OsStr::from_bytes(path.as_ref());
	let path = fs::Path::new(path_os);

	// List the ancestor paths.
	let mut n_ancestors = 0;
	let mut ancestors = [None; libc::PATH_MAX as usize];
	for (i, a) in path.ancestors().enumerate() {
		ancestors[i] = Some(a);
		n_ancestors = i + 1;
	}

	// Create the ancestor paths.
	for path_to_create in ancestors[0..n_ancestors].iter().rev() {
		let path_os = path_to_create.unwrap().as_os_str();

		let mut path_buf = [0u8; libc::PATH_MAX as usize];
		let path_c = join_c!(&mut path_buf, path_os.as_bytes());

		// If the directory already exists, then continue.
		let mut stat: MaybeUninit<libc::stat> = MaybeUninit::uninit();
		let ret = libc::stat(path_c.as_ptr(), stat.as_mut_ptr());
		if ret == 0 {
			let stat = stat.assume_init();
			if stat.st_mode & libc::S_IFDIR == libc::S_IFDIR {
				// If the ancestor is already a directory, then skip it.
				continue;
			}
		}

		let ret = libc::mkdir(path_c.as_ptr(), 0o777);
		if ret == -1 {
			return Err(std::io::Error::last_os_error());
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::ffi::CString;

	#[test]
	fn write_c() {
		let mut c_str_buf = [0u8; 128];
		let c_str: &CStr = write_c!(&mut c_str_buf, "/proc/{}/uid_map", 1_234);
		assert_eq!(
			c_str,
			CString::new("/proc/1234/uid_map").unwrap().as_c_str()
		);

		let c_str: &CStr = write_c!(&mut c_str_buf, "/proc/{}/uid_map", 123_456);
		assert_eq!(
			c_str,
			CString::new("/proc/123456/uid_map").unwrap().as_c_str()
		);
	}

	#[test]
	fn join_c() {
		use std::os::unix::ffi::OsStrExt;

		let mut buf = [0u8; 128];
		let first = String::from("first");
		let second = CString::new("second").unwrap();
		let third = std::path::PathBuf::from("third");
		let c_str = join_c!(
			&mut buf,
			first,
			"/",
			second.as_bytes(),
			"/",
			third.as_os_str().as_bytes()
		);

		assert_eq!(
			c_str,
			CString::new("first/second/third").unwrap().as_c_str()
		);
	}
}
