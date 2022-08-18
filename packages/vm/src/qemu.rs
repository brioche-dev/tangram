//! Create and manage `qemu`-backed virtual machines

#[macro_use]
pub mod cli;

use crate::bound_task;
use anyhow::{ensure, Context, Result};
use derive_more::From;
use std::{
	cmp,
	collections::BTreeMap,
	path::{Path, PathBuf},
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process;
use tracing::{debug, error, info, instrument};
use tracing_unwrap::ResultExt;
use ubyte::ByteUnit;

/// A VM's configuration
#[derive(Debug, Clone)]
pub struct Config {
	/// Path to the `qemu-system-ARCH` binary
	pub qemu_system: PathBuf,

	/// How to boot the guest: either with an EFI firmware image, or directly into a kernel image.
	pub boot: Boot,

	/// The number of CPU cores to give to the guest
	pub cores: usize,

	/// The amount of memory to give to the guest
	pub memory: ByteUnit,

	/// Block devices to attach to the guest
	pub drives: BTreeMap<DeviceName, Drive>,

	/// Host directories to share with the guest.
	pub shares: BTreeMap<DeviceName, Share>,

	/// Serial device to receive boot logs.
	pub boot_serial: Option<CharacterDevice>,

	/// Serial consoles to attach to the guest. These generally have `getty` behind them.
	pub serial_consoles: BTreeMap<DeviceName, CharacterDevice>,

	/// Serial ports (not consoles) to attach to the guest.
	pub serial_ports: BTreeMap<DeviceName, CharacterDevice>,

	/// Attach a QMP (QEMU Management Protocol) socket to the guest
	pub qmp_sock: Option<PathBuf>,
}

/// A guest boot source
#[derive(Debug, Clone, From)]
pub enum Boot {
	/// Path to an EFI firmware image to use to start the bootloader on an attached drive
	EfiFirmware(PathBuf),

	/// Directly boot a guest kernel
	Kernel(Kernel),
}

/// Options for direct kernel boot.
#[derive(Debug, Clone)]
pub struct Kernel {
	/// Path to the kernel image, compiled for the target architecture.
	pub image: PathBuf,

	/// Kernel commandline
	pub cmdline: String,
}

/// The name of a device, as identified to qemu
type DeviceName = String; // TODO: newtype restricted to ascii, 20char max

#[derive(Debug, Clone)]
pub struct Drive {
	/// The image file and format
	pub image: ImageFile,

	/// Whether to allow writes
	pub readonly: bool,

	/// Boot index
	/// The guest will attempt to boot drives with lower boot index first.
	/// If `None`, the system will attempt to boot the drive last.
	pub boot_index: Option<u64>,
}

/// A qemu virtfs share.
///
/// We use the device name given in the [`Config::shares`] map as the tag.
#[derive(Debug, Clone)]
pub struct Share {
	/// Host path to share with guest
	pub path: PathBuf,

	/// Whether to allow writes
	pub readonly: bool,
}

/// An image format supported by qemu for mounting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
	/// Raw image: a file with a disk image in it
	Raw,

	/// QEMU QCOW2 images
	Qcow2,
}

/// A qemu disk image, and the format it uses
#[derive(Debug, Clone)]
pub struct ImageFile {
	pub file: PathBuf,
	pub format: ImageFormat,
}

/// A qemu character device, exposed as a serial port to the guest.
#[derive(Debug, Clone)]
pub struct CharacterDevice {
	pub sock_file: PathBuf,
	pub log_file: Option<PathBuf>,
}

impl CharacterDevice {
	/// Represent the [`CharacterDevice`] as a qemu `-chardev`
	#[must_use]
	pub fn as_arg(&self, id: &str) -> cli::Arg {
		let mut chardev = arg!(
			-chardev,
			"socket",
			"id" = id,
			"path" = &self.sock_file,
			"server" = true,
			"wait" = false
		);
		if let Some(log) = &self.log_file {
			chardev.param("logfile", log);
		}
		chardev
	}
}

impl Config {
	/// Build a set of [`cli::Arg`]s from a qemu configuration
	#[instrument(name = "Config::as_args", skip_all)]
	pub fn as_args(&self) -> Result<Vec<cli::Arg>> {
		debug!(config = ?self, "converting qemu::Config to arguments");

		let mut args = vec![];
		let mut push = |arg: cli::Arg| {
			debug!(arg = ?arg);
			args.push(arg);
		};

		push(arg!(-name, "tangram-vm"));

		// Memory
		let memory_mb = (self.memory / ByteUnit::MB).as_u64();
		push(arg!(-m, memory_mb));

		// Configure guest CPU and machine type
		push(arg!(-cpu, "host"));
		push(arg!(-machine, "virt", "accel" = "hvf"));
		push(arg!(
			-smp,
			"sockets" = 1,
			"cores" = self.cores,
			"threads" = 1
		));

		// Configure the qemu boot process.
		match &self.boot {
			// Boot from an EFI firmware image.
			Boot::EfiFirmware(image) => {
				push(arg!(
					-drive,
					"if" = "pflash",
					"format" = "raw",
					"readonly" = "on",
					"file" = &image,
				));
			},

			// Directly boot a guest kernel.
			Boot::Kernel(kernel) => {
				push(arg!(-kernel, &kernel.image));
				push(arg!(-append, &kernel.cmdline));
			},
		};

		// Boot config:
		push(arg!(-boot, "splash-time" = 0, "menu" = true));

		// Attach drives
		for (name, drive) in &self.drives {
			ensure!(
				name.len() <= 20,
				"drive id {:?} is too long (max. 20 characters)",
				name
			);

			let drive_arg = arg!(
				-drive,
				"if" = "none", // we'll attach the `-device` ourselves
				"id" = &name,
				"file" = &drive.image.file,
				"format" = drive.image.format,
				"readonly" = drive.readonly,
			);
			let mut device_arg = arg!(
				-device,
				"virtio-blk-pci",
				"drive" = &name,
				// use the drive ID as its serial number, so it can be
				// unambiguously mounted inside the guest.
				//
				// This will have the effect of creating the symlink
				// `/dev/disk/by-label/virtio-SERIAL`.
				"serial" = &name,
			);
			if let Some(idx) = drive.boot_index {
				device_arg.param("bootindex", idx);
			}

			push(drive_arg);
			push(device_arg);
		}

		// Attach the network device
		push(arg!(
			-netdev,
			"user",
			"id" = "net0",
			"net" = "192.168.5.0/24",
			"dhcpstart" = "192.168.5.15",
			"hostfwd" = "tcp:127.0.0.1:60664-:22",
		));
		push(arg!(
			-device,
			"virtio-net-pci",
			"netdev" = "net0",
			"mac" = "52:55:55:95:15:70",
			// Don't require 'efi-virtio.rom' to be present to run the VM.
			// We do this by disabling the network device's Option ROM, which is
			// only required for PXE boot (which we aren't using).
			"romfile" = "",
		));

		// Attach any file shares
		for (name, share) in &self.shares {
			let mut virtfs = arg!(
				-virtfs,
				"local",
				"path" = &share.path,
				"mount_tag" = &name,
				"security_model" = "none",
				"multidevs" = "forbid",
			);
			if share.readonly {
				virtfs.param("readonly", true);
			}
			push(virtfs);
		}

		// Attach RNG device
		push(arg!(-device, "virtio-rng-pci"));

		// Disable displays and parallel port
		push(arg!(-display, "none"));
		push(arg!(-vga, "none"));
		push(arg!(-parallel, "none"));

		if let Some(console) = &self.boot_serial {
			let id = "serial-console";
			push(console.as_arg(id));
			push(arg!(-serial, format!("chardev:{id}")));
		}

		// Attach the virtio-serial driver for serial ports.
		push(arg!(
			-device,
			"virtio-serial",
			"max_ports" = cmp::max(32, self.serial_ports.len() + self.serial_consoles.len())
		));

		// Attach console ports.
		for (id, port) in &self.serial_consoles {
			push(port.as_arg(id));
			push(arg!(
				-device,
				"virtconsole",
				"chardev" = &id,
				"id" = &id,
				"name" = format!("dev.tangram.serial.{}", id)
			));
		}

		// Attach serial ports.
		for (id, port) in &self.serial_ports {
			push(port.as_arg(id));
			push(arg!(
				-device,
				"virtserialport",
				"chardev" = &id,
				"id" = &id,
				"name" = format!("dev.tangram.serial.{}", id)
			));
		}

		// Configure QMP (QEMU Management Protocol) socket
		if let Some(qmp_sock) = &self.qmp_sock {
			push(arg!(
				-chardev,
				"socket",
				"id" = "char-qmp",
				"path" = &**qmp_sock,
				"server" = true,
				"wait" = false
			));
			push(arg!(-qmp, "chardev:char-qmp"));
		}

		Ok(args)
	}
}

/// A running qemu instance.
pub struct Qemu {
	child: process::Child,
	_stderr_forwarder: bound_task::BoundJoinHandle<()>,
	_stdout_forwarder: bound_task::BoundJoinHandle<()>,
}

impl Qemu {
	/// Build a [`tokio::process::Command`] from the qemu configuration and the path to a
	/// `qemu-system-ARCH` binary.
	#[instrument(name = "Qemu::spawn", skip_all)]
	pub fn spawn(config: &Config, qemu_system: &Path) -> Result<Qemu> {
		use std::process::Stdio;

		// Build the invocation.
		let mut cmd = process::Command::new(qemu_system);
		cmd.kill_on_drop(true);
		for arg in config.as_args()? {
			cmd.args(arg.as_argv());
		}

		// Configure all streams as piped.
		cmd.stdin(Stdio::piped());
		cmd.stdout(Stdio::piped());
		cmd.stderr(Stdio::piped());

		// Start the qemu subprocess.
		let mut child = cmd.spawn().context("failed to start qemu subprocess")?;

		// Forward qemu stderr lines to tracing.
		let stderr = child
			.stderr
			.take()
			.expect("child did not have piped stderr");
		let stderr_forwarder =
			bound_task::spawn(async { forward_qemu_stderr(stderr).await.unwrap_or_log() });

		// Forward qemu stdout lines to tracing.
		let stdout = child
			.stdout
			.take()
			.expect("child did not have piped stdout");
		let stdout_forwarder =
			bound_task::spawn(async { forward_qemu_stdout(stdout).await.unwrap_or_log() });

		Ok(Qemu {
			child,
			_stderr_forwarder: stderr_forwarder,
			_stdout_forwarder: stdout_forwarder,
		})
	}

	/// Wait for the qemu instance to shut down
	///
	/// # Errors
	/// Fails if we fail to wait for the child process to exit, for some reason.
	#[instrument(name = "Qemu::wait_for_shutdown", skip_all)]
	pub async fn wait_for_shutdown(mut self) -> Result<()> {
		self.child
			.wait()
			.await
			.context("child did not exit cleanly")?;
		Ok(())
	}

	/// Forcefully kill the qemu instance
	///
	/// # Errors
	/// Returns an error if we fail to kill the child process
	#[instrument(name = "Qemu::kill", skip_all)]
	pub async fn kill(mut self) -> Result<()> {
		self.child
			.kill()
			.await
			.context("failed to kill qemu instance")?;
		Ok(())
	}
}

#[instrument(name = "forward_qemu_stderr", skip_all)]
async fn forward_qemu_stderr(stderr: process::ChildStderr) -> Result<()> {
	let mut stderr_reader = BufReader::new(stderr).lines();
	while let Some(line) = stderr_reader.next_line().await? {
		error!(line = line.as_str());
	}
	Ok(())
}

#[instrument(name = "forward_qemu_stdout", skip_all)]
async fn forward_qemu_stdout(stderr: process::ChildStdout) -> Result<()> {
	let mut stderr_reader = BufReader::new(stderr).lines();
	while let Some(line) = stderr_reader.next_line().await? {
		info!(line = line.as_str());
	}
	Ok(())
}
