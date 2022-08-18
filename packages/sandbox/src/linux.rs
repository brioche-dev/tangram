#![cfg(target_os = "linux")] // All this code is Linux-specific.

use crate::{Sandbox, Writability};
use ::unshare::Command;
use anyhow::{ensure, Context, Result};
use nix::{
	self,
	mount::{mount, umount2, MntFlags, MsFlags},
};
use scopeguard::defer;
use std::{
	cell::RefCell,
	ffi::{OsStr, OsString},
	fmt, fs,
	os::unix::ffi::OsStrExt,
	path::{Path, PathBuf},
};

/// Configuration for Linux sandboxing.
#[derive(Debug)]
pub struct Sandbox {
	/// Change root to the given directory.
	pub root_dir: PathBuf,

	/// Set the child's working directory inside the sandbox.
	pub workdir: Option<PathBuf>,

	/// List of paths to bind-mount into the sandbox.
	pub bind_mounts: Vec<BindMount>,

	/// Detach the sandbox from the host's networking namespace
	pub isolate_network: bool,

	/// Path to a directory where the sandbox can safely create scratch files and directories.
	pub resource_dir: TempDir,
}

/// [`BindMount`] describes a bind mount from the host into the sandbox.
#[derive(Clone, Debug, PartialEq)]
pub struct BindMount {
	/// The path to the mount contents on the host
	pub outer: PathBuf,

	/// The path to the mount contents inside the sandbox
	pub inner: PathBuf,

	/// Whether the mount is read-write or read-only
	pub write: Writability,
}

/// Whether a mount is read-write or read-only
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Writability {
	ReadOnly,
	ReadWrite,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ExitStatus {
	Code(i32),
	Signal(i32),
}

impl ExitStatus {
	/// Did the subprocess exit successfully?
	pub fn success(&self) -> bool {
		matches!(self, ExitStatus::Code(0))
	}
}

impl Sandbox {
	/// Spawn a command inside the sandbox
	pub fn spawn(&self, cmd: &str, args: &[&str]) -> Result<unshare::ExitStatus> {
		// We want an absolute path to the root directory.
		let root_source_path = self
			.root_dir
			.canonicalize()
			.context("failed to canonicalize sandbox root path")?;

		// Create a directory for overlayfs to store any changes made in the sandbox.
		// We don't want to modify the actual contents of the root_dir on disk, but we
		// do want to allow the sandboxed child to make changes.
		let overlay_changes_path = self.resource_dir.path().join("overlayfs_changes");
		fs::create_dir(&overlay_changes_path)
			.context("could not create tempdir to store fs overlay")?;

		// Overlayfs requires a temp dir to stage files before atomically moving them
		// into the upper layer (to e.g. represent deletions).
		let overlay_work_dir_path = self.resource_dir.path().join("overlayfs_work");
		fs::create_dir(&overlay_work_dir_path)
			.context("coult not create tempdir to store overlayfs workdir")?;

		// Make a directory into which we overlayfs-mount the root source directory.
		let root_path = self.resource_dir.path().join("root");
		fs::create_dir(&root_path)
			.context("coult not create tempdir to store sandbox overlayfs root")?;

		// Keep track of all the mounts we make, so we can unmount them all cleanly.
		let mount_points_to_unmount = RefCell::new(Vec::<PathBuf>::new());
		let unmount_on_exit =
			|path: &Path| mount_points_to_unmount.borrow_mut().push(path.to_owned());
		defer! {
			// Reverse-iterate, so we get to inner mounts before outer ones.
			for mount_point in mount_points_to_unmount.borrow().iter().rev() {
				// We unwrap() here because we can't reasonably recover from errors.
				// MNT_DETACH: Lazy unmount, removing from filesystem even if busy, cleaning up
				// later.
				umount2(mount_point.as_os_str(), MntFlags::MNT_DETACH)
					.with_context(|| format!("failed to unmount: {:?}", mount_point))
					.unwrap();
			}
		}

		// Create the overlayfs mount
		let mount_opts = &[
			("lowerdir", &root_source_path),     // Read-only access to the root
			("upperdir", &overlay_changes_path), // Any changes go in the tempdir
			("workdir", &overlay_work_dir_path),
		];
		let mount_opt_string = make_mount_option_string(mount_opts)?;
		mount(
			None::<&Path>,    // Source (in this case, a dummy string, not a device)
			&root_path,       // Target (the path we're mounting into)
			Some("overlay"),  // Filesystem type
			MsFlags::empty(), // Mount flags
			Some(mount_opt_string.as_os_str()), // Option string (void* data)
		)
		.context("could not mount overlayfs")?;
		unmount_on_exit(&root_path);

		// Mount all binds inside the overlayfs
		for bind in &self.bind_mounts {
			let outer = bind.outer.canonicalize()?;
			let inner = &bind.inner;
			ensure!(
				inner.is_absolute(),
				"bind mount inner path must be absolute"
			);

			// Re-parent the inner path within the root_path
			let inner_inside_root = root_path.join(
				inner
					.strip_prefix("/")
					.expect("could not remove '/' prefix from absolute path"),
			);

			// Create the inner path as a directory
			fs::create_dir_all(&inner_inside_root)
				.context("could not create mount point dir for bind mount")?;

			let mount_with_flags = |flags: MsFlags| {
				mount(
					Some(outer.as_path()),       // Source (the host path)
					inner_inside_root.as_path(), // Target (the path inside the sandbox)
					None::<&OsStr>,              // No filesystem type, it's a bind mount
					flags,                       // Bind options
					None::<&OsStr>,              // No options
				)
			};

			// Try to mount the first time with just MS_BIND
			mount_with_flags(MsFlags::MS_BIND).with_context(|| {
				format!(
					"could not bind mount '{:?}' into '{:?}'",
					bind.outer, inner_inside_root
				)
			})?;

			// If this is a read-only mount, remount with MS_RDONLY as well
			// You need to remount in this case---without remounting, even if you pass MS_RDONLY,
			// the bind will still appear as read-write.
			if bind.write == Writability::ReadOnly {
				let flags = MsFlags::MS_REMOUNT | MsFlags::MS_BIND | MsFlags::MS_RDONLY;

				mount_with_flags(flags).with_context(|| {
					format!(
						"could not remount bind mount {:?}:{:?} as read-only",
						bind.outer, inner_inside_root
					)
				})?;
			} else {
			}

			unmount_on_exit(&inner_inside_root);
		}

		let mut cmd = Command::new(cmd);
		cmd.args(args);

		// `chroot()` inside the child to the overlayfs mount
		cmd.chroot_dir(&root_path);

		// Set the working directory inside the child
		let cwd = self
			.workdir
			.as_deref()
			.unwrap_or_else(|| Path::new("/"))
			.to_owned();
		cmd.current_dir(&cwd);

		// Unshare namespaces
		if self.isolate_network {
			// Isolate network if requested
			cmd.unshare(&[unshare::Namespace::Net]);
		}
		cmd.unshare(&[
			unshare::Namespace::Pid,   // Isolate processes from one another
			unshare::Namespace::Mount, // Let us separately mount /proc
			unshare::Namespace::Uts,   // Container gets its own namespace
			unshare::Namespace::Ipc,   // for SysV IPC, message queues
		]);

		// Mount /proc
		let proc_inside_root = root_path.join("proc");
		fs::create_dir_all(&proc_inside_root)
			.context("could not create /proc mountpoint inside sandbox")?;
		unsafe {
			cmd.pre_exec(move || {
				mount(
					None::<&OsStr>,   // Source (none)
					"/proc",          // Destination (container /proc)
					Some("proc"),     // procfs
					MsFlags::empty(), // Mount flags
					None::<&OsStr>,   // No mount options
				)?;
				Ok(())
			});
		}

		// Execute the child
		let result = cmd
			.status()
			.map_err(UnshareErr)
			.context("failed to spawn sandboxed child")?;

		Ok(result)
	}
}

#[derive(Debug)]
pub struct UnshareErr(unshare::Error);
impl std::error::Error for UnshareErr {}
impl fmt::Display for UnshareErr {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.0.fmt(f)
	}
}

/// Format the option string for `mount`, as comma separated `key=value`.
fn make_mount_option_string<K, V>(values: &[(K, V)]) -> Result<OsString>
where
	K: AsRef<OsStr>,
	V: AsRef<OsStr>,
{
	let check_valid_chars = |s: &OsStr| -> Result<()> {
		let bytes = s.as_bytes();
		ensure!(!bytes.contains(&b':'), "mount option cannot contain ':'");
		ensure!(!bytes.contains(&b'='), "mount option cannot contain '='");
		ensure!(!bytes.contains(&b','), "mount option cannot contain ','");
		Ok(())
	};

	let mut opts = OsString::new();
	for (k, v) in values {
		let k = k.as_ref();
		let v = v.as_ref();

		check_valid_chars(k)?;
		check_valid_chars(v)?;

		opts.push(k);
		opts.push("=");
		opts.push(v);
		opts.push(",");
	}
	Ok(opts)
}
