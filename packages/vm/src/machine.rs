use crate::{
	agent,
	bound_task::{self},
	cloud_init,
	macos::{
		foundation::DispatchQueue,
		vz::{
			virtualization_supported, VZDirectorySharingDeviceConfiguration,
			VZDiskImageCachingMode, VZDiskImageStorageDeviceAttachment,
			VZDiskImageSynchronizationMode, VZFileSerialPortAttachment, VZLinuxBootLoader,
			VZMACAddress, VZNATNetworkDeviceAttachment, VZSharedDirectory, VZSingleDirectoryShare,
			VZStorageDeviceConfiguration, VZVirtioBlockDeviceConfiguration,
			VZVirtioConsoleDeviceSerialPortConfiguration, VZVirtioEntropyDeviceConfiguration,
			VZVirtioFileSystemDeviceConfiguration, VZVirtioNetworkDeviceConfiguration,
			VZVirtioSocketConnection, VZVirtioSocketDevice, VZVirtioSocketDeviceConfiguration,
			VZVirtualMachine, VZVirtualMachineConfiguration, VsockStream,
		},
	},
	mem_info::MemInfo,
	systemd_unit,
	template::Template,
	Writability,
};
use anyhow::{ensure, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use derive_more::Display;
use std::{collections::BTreeMap, path::Path, time::Duration};
use tempfile::TempDir;
use tokio::io::AsyncReadExt;
use tokio::{fs, net::UnixListener};
use tracing_unwrap::ResultExt;

use tokio::time::{sleep, timeout};
use tracing::{debug, error, info, info_span, instrument, trace, Instrument, Span};

/// Default name for a Machine.
/// This is set as the machine's hostname, the qemu vm name, and the cloud-init instance ID.
pub const DEFAULT_MACHINE_NAME: &str = "tangram-vm";

/// Port on which the guest agent listens for connections from the host.
const GUEST_AGENT_VSOCK_PORT: u32 = 42_42_42_42_01;

/// First port used for forwarding Unix connectiont to the guest.
/// We allocate ports sequentially, starting at this constant, and increasing by one for each
/// socket forwarded.
const FORWARD_PORT_RANGE_START: u32 = 42_42_42_42_02;

/// Configuration to create a [`Machine`].
#[derive(Debug)]
pub struct Builder {
	/// Distributable artifact, with boot image, kernel, and configuration
	template: Template,

	/// Extra disk images to mount
	disks: Vec<Disk>,

	/// Extra users to create in the guest machine
	users: Vec<User>,

	/// Extra groups to create in the guest machine
	groups: Vec<Group>,

	/// Shared folders to be mapped into the guest
	shares: Vec<Share>,

	/// Unix sockets to forward into the guest
	sockets: Vec<Socket>,

	/// Include extra files in the cloud-init image.
	extra_files: BTreeMap<String, Vec<u8>>,
}

pub struct Machine {
	/// The [`VZVirtualMachine`] instance itself
	vm: VZVirtualMachine,

	/// Span for VM-related tracing events
	span: tracing::Span,

	/// A directory containing VM runtime resources
	dir: RunDir,

	/// The virtio socket device attached to the guest
	vsock_device: VZVirtioSocketDevice,

	/// Unix sockets forwarded into the guest
	sockets: Vec<Socket>,

	/// Tasks to forward unix connections through vsock to the guest.
	_socket_forward_tasks: Vec<bound_task::BoundJoinHandle<()>>,
}

impl Builder {
	/// Create a new `MachineBuilder`.
	pub fn new(template: Template) -> Result<Builder> {
		Ok(Builder {
			template,
			disks: vec![],
			users: vec![],
			groups: vec![],
			shares: vec![],
			sockets: vec![],
			extra_files: BTreeMap::new(),
		})
	}

	/// Add a disk image mount to the machine.
	pub fn add_disk(&mut self, disk: Disk) -> &mut Self {
		self.disks.push(disk);
		self
	}

	/// Add a shared directory to the machine.
	pub fn add_share(&mut self, share: Share) -> &mut Self {
		self.shares.push(share);
		self
	}

	/// Add a socket to be forwarded to the machine.
	pub fn add_socket(&mut self, socket: Socket) -> &mut Self {
		self.sockets.push(socket);
		self
	}

	/// Add a user to the machine
	pub fn add_user(&mut self, user: User) -> &mut Self {
		self.users.push(user);
		self
	}

	/// Add a group to the machine
	pub fn add_group(&mut self, group: Group) -> &mut Self {
		self.groups.push(group);
		self
	}

	/// Add a file to be baked into the cloud-init image.
	pub fn add_init_file(&mut self, name: &str, file: Vec<u8>) -> &mut Self {
		self.extra_files.insert(name.to_owned(), file);
		self
	}

	/// Start the machine
	#[instrument(name = "Start Machine", skip_all)]
	pub async fn start(self) -> Result<Machine> {
		assert!(
			virtualization_supported(),
			"Apple Virtualization is not supported on this platform"
		);

		// Create a tracing span for the VM's lifetime
		let vm_span = info_span!(parent: None, "VM");
		vm_span.follows_from(Span::current());

		// Create a VM resource dir.
		let dir = RunDir::new()
			.await
			.context("failed to create VM resource directory")?;
		info!(dir=%dir.path());

		// Snapshot the boot image into the run dir.
		tokio::fs::copy(self.template.boot_disk_image, dir.boot_image())
			.await
			.context("failed to copy boot image to run dir")?;

		// Create blank cloud-init data.
		let mut init_data = cloud_init::InitData::new(DEFAULT_MACHINE_NAME);

		// Move over extra files from the builder.
		init_data.extra_files = self.extra_files;

		// Examine the kernel image. Apple VZ doesn't support compressed kernel images, and fails
		// with an absolutely useless error ("Internal Virtualization error") if you try to
		// pass one. Here, we check for the gzip magic number, and if we see it, we'll fail
		// with a less useless error.
		let mut f = fs::File::open(&self.template.kernel)
			.await
			.context("failed to open kernel image to check header")?;
		let mut header = [0u8; 2];
		f.read_exact(&mut header)
			.await
			.context("failed to read first few bytes of kernel image while checking header")?;
		trace!(header = %format!("{header:x?}"), "sniffed kernel header");
		ensure!(
			header != [0x1f, 0x8b], // Gzip magic number
			"Kernel image is gzip-compressed, which Virtualization.framework does not support"
		);

		// Set up the VZLinuxBootLoader according to the template.
		let mut bootloader = VZLinuxBootLoader::with_kernel(&self.template.kernel);
		bootloader.set_command_line(&self.template.kernel_command_line);
		if let Some(initrd) = &self.template.initrd {
			bootloader.set_initial_ramdisk(initrd);
		}
		debug!(?bootloader, kernel=%self.template.kernel, initrd=?self.template.initrd, cmdline=%self.template.kernel_command_line);

		// Configure a serial port attached to stdio
		let serial_attachment = VZFileSerialPortAttachment::new_truncate(&dir.console_log())
			.context("failed to open serial port logfile while creating serial port")?;
		let serial_port =
			VZVirtioConsoleDeviceSerialPortConfiguration::with_attachment(&serial_attachment);
		trace!(?serial_attachment, ?serial_port);

		// Configure a boot disk
		let disk_attachment = VZDiskImageStorageDeviceAttachment::from_cache_config(
			&dir.boot_image(),
			Writability::ReadWrite,
			VZDiskImageCachingMode::Cached,
			VZDiskImageSynchronizationMode::Fsync,
		)
		.context("failed to create disk image storage attachment")?;

		let mut boot_disk_block_device =
			VZVirtioBlockDeviceConfiguration::with_attachment(&disk_attachment);
		boot_disk_block_device
			.set_identifier("tangram-boot")
			.context("failed to set boot drive identifier")?;
		trace!(?boot_disk_block_device, ?disk_attachment);

		// Configure the other block devices
		let mut other_disks = vec![];
		for disk in self.disks {
			// Attach a block device to the VM
			let attachment = VZDiskImageStorageDeviceAttachment::from_cache_config(
				&disk.image,
				disk.access,
				VZDiskImageCachingMode::Cached,
				VZDiskImageSynchronizationMode::Fsync,
			)
			.with_context(|| format!("failed to create disk attachment: image '{}'", disk.image))?;
			let mut block_device = VZVirtioBlockDeviceConfiguration::with_attachment(&attachment);
			block_device
				.set_identifier(&disk.id)
				.with_context(|| format!("invalid identifier: disk '{}'", disk.id))?;
			other_disks.push(block_device);

			// Configure a mount for that block device
			if let Some(mount) = disk.mount {
				init_data.user_data.mounts.push(vec![
					format!("/dev/disk/by-id/virtio-{}", disk.id),
					mount.mountpoint.to_string(),
					mount.fs.to_string(),
					format!(
						"{},x-systemd.makefs,x-systemd.growfs,x-systemd.after=cloud-init.service",
						disk.access.as_ro_rw()
					),
					"0".into(),
					"0".into(),
				]);

				// If the mount uses btrfs, make sure btrfs-progs is installed
				if mount.fs == DiskFs::Btrfs {
					init_data.user_data.packages.push("btrfs-progs".to_string());
				}
			}
		}

		// Configure a NAT network device.
		let net_attachment = VZNATNetworkDeviceAttachment::new();
		let net_mac = VZMACAddress::random_local_address();
		let mut net_device = VZVirtioNetworkDeviceConfiguration::with_attachment(&net_attachment);
		net_device.set_mac_address(&net_mac);
		trace!(?net_device, ?net_mac);

		// Configure an entropy device
		let entropy_device = VZVirtioEntropyDeviceConfiguration::new();
		trace!(?entropy_device);

		// Configure a virtio socket device
		let virtio_socket = VZVirtioSocketDeviceConfiguration::new();
		trace!(?virtio_socket);

		// Configure users with cloud-init
		for user in self.users {
			debug!(
				name=?user.name,
				uid=?user.uid,
				password.is_some=user.password.is_some(),
				sudo=?user.sudo,
				primary_group=?user.primary_group,
				groups=?user.groups,
				ssh_authorized_keys=?user.ssh_authorized_keys,
				"Configure VM user"
			);
			init_data.user_data.users.push(cloud_init::User {
				name: user.name,
				uid: user.uid,
				lock_passwd: Some(user.password.is_none()),
				plain_text_passwd: user.password,
				sudo: user.sudo.then(|| cloud_init::PASSWORDLESS_SUDO.to_owned()),
				primary_group: user.primary_group,
				groups: user.groups,
				ssh_authorized_keys: user.ssh_authorized_keys,
			});
		}

		// Configure groups with cloud-init
		for group in self.groups {
			debug!(
				name=?group.name,
				gid=?group.gid,
				"Configure VM group"
			);

			// Create the group with cloud-init
			init_data.user_data.groups.insert(group.name.clone());

			// Configure the gid manually, cloud-init won't do this for us.
			if let Some(gid) = group.gid {
				init_data.user_data.run_commands.push(vec![
					"groupmod".into(),
					"-g".into(),
					format!("{gid}"),
					group.name,
				]);
			}
		}

		// Configure shared folders
		let mut virtfs_devices = vec![];
		for share in self.shares {
			debug!(
				host_path=?share.host_path,
				guest_path=?share.guest_path,
				readonly=%share.access,
				tag=?share.tag,
				"Configure VM shared directory",
			);

			// Configure the virtiofs device
			let dir = VZSharedDirectory::new(&share.host_path, share.access);
			let dir_share = VZSingleDirectoryShare::new(&dir);
			let mut device = VZVirtioFileSystemDeviceConfiguration::new(&share.tag)?;
			device.set_share(&dir_share);
			virtfs_devices.push(device);

			// Add the cloud_init fstab entry
			init_data.user_data.mounts.push(vec![
				share.tag.clone(),
				share.guest_path.to_string(),
				"virtiofs".into(),
				share.access.as_ro_rw().to_string(),
				"0".into(),
				"0".into(),
			]);
		}

		// Add the guest agent binary to the cloud-init data image.
		let guest_agent_bytes = tokio::fs::read(&self.template.guest_agent)
			.await
			.context("failed to read guest agent binary from template")?;
		init_data
			.extra_files
			.insert("tangram-guest-agent".into(), guest_agent_bytes);

		// Define a systemd unit for the guest agent, so it runs on boot.
		let mut exec_str = String::from("/mnt/init_data/tangram-guest-agent");
		exec_str.push_str(&format!(" --port {GUEST_AGENT_VSOCK_PORT}"));
		for (i, sock) in self.sockets.iter().enumerate() {
			// Add a --forward-unix argument for each socket to forward
			ensure!(
				!sock.guest_path.as_str().contains(':'),
				"guest socket paths cannot contain ':'"
			);
			let port = FORWARD_PORT_RANGE_START + u32::try_from(i).unwrap();
			let path = &sock.guest_path;
			let arg = format!(" --forward-unix '{port}:{path}'");
			exec_str.push_str(&arg);
		}

		let service = systemd_unit::ServiceUnit {
			unit: systemd_unit::Unit {
				description: "Tangram VM Guest Agent".into(),
				requires_mounts_for: vec!["/mnt/init_data".into()],
				after: vec!["cloud-init.target".into()],
				before: vec!["default.target".into()],
				..Default::default()
			},
			service: systemd_unit::Service {
				exec_start: Some(exec_str),
				kind: Some(systemd_unit::ServiceType::Exec),
				restart: Some(systemd_unit::ServiceRestart::OnFailure),
				standard_input: None,
				standard_output: Some("null".to_string()),
				standard_error: Some("journal".to_string()),
			},
			install: systemd_unit::Install {
				wanted_by: vec!["default.target".into()],
				..Default::default()
			},
		};

		// Create a systemd service to run the guest agent
		init_data.user_data.write_files.push(cloud_init::WriteFile {
			path: "/etc/systemd/system/tangram-guest-agent.service".into(),
			owner: Some("root:root".into()),
			content: service
				.to_conf()
				.context("failed to serialize systemd service config")?,
			encoding: None,
		});

		// Enable the service on first boot
		init_data.user_data.run_commands.push(vec![
			"systemctl".into(),
			"enable".into(),
			"--now".into(),
			"--no-block".into(), // Don't hold up completion of cloud-init waiting for the service
			"tangram-guest-agent.service".into(),
		]);

		// Mount the cloud-init image on boot.
		init_data.user_data.mounts.push(vec![
			"/dev/disk/by-id/virtio-cloud-init-data".into(),
			"/mnt/init_data".into(),
			"vfat".into(),
			"ro".into(),
			"0".into(), // don't dump
			"0".into(), // don't fsck
		]);

		// Save the cloud-init data, attach a block device with it to the VM.
		let init_fatfs_image = init_data
			.into_disk_image()
			.context("failed to build cloud-init fatfs image")?;
		fs::write(&dir.cloud_init_image(), init_fatfs_image)
			.await
			.context("failed to write cloud-init image to disk")?;
		let init_data_attachment =
			VZDiskImageStorageDeviceAttachment::new(dir.cloud_init_image(), Writability::ReadOnly)
				.context("failed to create attachment for cloud-init image")?;
		let mut init_data_block_device =
			VZVirtioBlockDeviceConfiguration::with_attachment(&init_data_attachment);
		init_data_block_device
			.set_identifier("cloud-init-data")
			.context("failed to set disk identifier for cloud-init fatfs image")?;
		trace!(?init_data_block_device, ?init_data_attachment);

		// Give all the host's available parallelism to the guest
		let guest_cpus: usize = std::thread::available_parallelism()
			.map(Into::into)
			.unwrap_or(4);
		debug!(guest_cpus);

		// Configure the guest's *hardware* to expand to all the host's memory
		let guest_max_mem = (75 * MemInfo::measure().total) / 100;
		debug!(%guest_max_mem);

		// Configure and create the VZVirtualMachine
		let storage_devices = [&boot_disk_block_device, &init_data_block_device]
			.into_iter()
			.chain(other_disks.iter())
			.map(|d| d as &dyn VZStorageDeviceConfiguration)
			.collect::<Vec<_>>();
		let directory_sharing_devices = virtfs_devices
			.iter()
			.map(|d| d as &dyn VZDirectorySharingDeviceConfiguration)
			.collect::<Vec<_>>();
		let mut vm_config = VZVirtualMachineConfiguration::new();
		vm_config
			.set_cpu_count(guest_cpus)
			.set_memory_size(guest_max_mem)
			.set_boot_loader(&bootloader)
			.set_socket_devices(&[&virtio_socket])
			.set_serial_ports(&[&serial_port])
			.set_entropy_devices(&[&entropy_device])
			.set_network_devices(&[&net_device])
			.set_directory_sharing_devices(&directory_sharing_devices)
			.set_storage_devices(&storage_devices);
		debug!(?vm_config);
		let vm = VZVirtualMachine::new(&vm_config).expect("failed to create virtual machine");
		let vm_queue = vm.dispatch_queue();
		debug!(?vm);

		// Start the guest (10s timeout, usually ~50ms)
		timeout(Duration::from_secs(10), vm.start())
			.instrument(info_span!(parent: &vm_span, "Start guest"))
			.await
			.context("VM timed out while starting")?
			.context("failed to start VM")?;

		// Get the socket device from the running VM
		let vm_socket: VZVirtioSocketDevice = vm
			.socket_devices()
			.pop()
			.context("VM did not create a Virtio socket device")?
			.try_into()
			.context("VM created the wrong kind of socket device")?;
		trace!(?vm_socket);

		// Boot the guest (5m timeout, usually ~10s)
		let conn = timeout(Duration::from_secs(60 * 5), async {
			loop {
				let attempt = vm_socket
					.connect_to_port(&vm_queue, GUEST_AGENT_VSOCK_PORT)
					.await;
				if let Ok(conn) = attempt {
					break conn;
				}
				sleep(Duration::from_millis(50)).await;
			}
		})
		.instrument(info_span!(parent: &vm_span, "Wait for guest agent"))
		.await
		.context("VM timed out while booting")?;
		info!(parent: &vm_span, "Online");

		// Handshake with the guest agent (make sure the versions match up)
		let conn_stream = conn
			.into_stream()
			.context("failed to open stream to guest agent")?;
		agent::host::Client::connect(conn_stream)
			.await
			.context("Failed to connect to guest agent")?;

		// Start socket forwarding tasks
		let mut forwarder_tasks = vec![];
		for (i, sock) in self.sockets.iter().enumerate() {
			// Start a forwarder task for each socket.

			forwarder_tasks.push({
				let path = dir.forwarded_socket_path(sock);
				let device = vm_socket.clone();
				let queue = vm.dispatch_queue().clone();
				let port = FORWARD_PORT_RANGE_START + u32::try_from(i).unwrap();
				let span = vm_span.clone();
				bound_task::spawn(forward_unix_to_guest(device, queue, path, port, span))
			});
		}

		Ok(Machine {
			vm,
			dir,
			vsock_device: vm_socket,
			span: vm_span,
			sockets: self.sockets,
			_socket_forward_tasks: forwarder_tasks,
		})
	}
}

#[instrument(level="INFO", name="Forward unix to vsock", parent=span, skip_all, fields(%socket_path, %vsock_port))]
async fn forward_unix_to_guest(
	vsock_device: VZVirtioSocketDevice,
	vm_queue: DispatchQueue,
	socket_path: Utf8PathBuf,
	vsock_port: u32,
	span: tracing::Span,
) {
	let listener = UnixListener::bind(&socket_path).expect_or_log("failed to bind unix listener");
	loop {
		let (mut unix_stream, _addr) = match listener.accept().await {
			Ok(x) => x,
			Err(e) => {
				error!(err=%e, "Failed to accept connection");
				continue;
			},
		};

		debug!("Accepted connection from unix");

		// Connect to the vsock port
		let mut vsock_stream = match vsock_device
			.connect_to_port(&vm_queue, vsock_port)
			.await
			.map(VZVirtioSocketConnection::into_stream)
		{
			Ok(Ok(x)) => x,
			Ok(Err(e)) => {
				error!(err=%e, "Failed to initialize vsock stream");
				continue;
			},
			Err(e) => {
				error!(err=%e, "Failed to connect to vsock");
				continue;
			},
		};

		debug!("Connected to vsock, proxying connections.");

		// Plug the connections into each other
		tokio::task::spawn(
			async move {
				tokio::io::copy_bidirectional(&mut unix_stream, &mut vsock_stream)
					.await
					.map_or_else(|e| error!(err=%e, "Failed to proxy connections"), |_| ());
				debug!("Connection completed");
			}
			.in_current_span(),
		);
	}
}

impl Machine {
	/// Forcibly kill the virtual machine.
	pub async fn kill(self) -> Result<()> {
		self.vm.kill().await?;
		info!(parent: &self.span, "Killed");
		Ok(())
	}

	/// Wait for the VM to power off.
	#[instrument(skip_all, parent=&self.span, name = "Waiting for power off")]
	pub async fn wait(self) -> Result<()> {
		self.vm
			.wait()
			.await
			.context("failed to wait for guest power off")?;
		Ok(())
	}

	/// Shut down the guest virtual machine, waiting for it to power off.
	#[instrument(skip_all, parent=&self.span, name = "Shutting down")]
	pub async fn shutdown(self) -> Result<()> {
		self.vm
			.request_shutdown()
			.await
			.context("failed to request guest shutdown")?;
		self.vm
			.wait()
			.await
			.context("failed to wait for guest power off")?;
		Ok(())
	}

	/// Get the path to a forwarded socket with the given name.
	#[must_use]
	pub fn path_to_socket(&self, name: &str) -> Option<Utf8PathBuf> {
		let sock = self.sockets.iter().find(|s| s.name == name)?;
		Some(self.dir.forwarded_socket_path(sock))
	}

	/// Connect to the VM's guest agent.
	pub async fn agent(&self) -> Result<agent::host::Client> {
		// Connect to the guest agent over VSock
		let dispatch_queue = self.vm.dispatch_queue().clone();
		let conn = self
			.vsock_device
			.connect_to_port(&dispatch_queue, GUEST_AGENT_VSOCK_PORT)
			.await
			.context("failed to connect to guest agent VSock port")?;
		let conn_stream = conn
			.into_stream()
			.context("failed to create stream from VSock connection")?;

		// Handshake with the guest agent.
		let client = agent::host::Client::connect(conn_stream)
			.await
			.context("failed to handshake with guest agent")?;

		Ok(client)
	}

	/// Connect to a vsock port on the VM.
	pub async fn connect_vsock(&self, port: u32) -> Result<VsockStream> {
		let dispatch_queue = self.vm.dispatch_queue().clone();
		let conn = self
			.vsock_device
			.connect_to_port(&dispatch_queue, port)
			.await
			.context("failed to connect to vsock port")?;
		let conn_stream = conn
			.into_stream()
			.context("failed to create stream from vsock connection")?;
		Ok(conn_stream)
	}
}

#[derive(Debug)]
pub struct RunDir {
	_tempdir: TempDir,
	path: Utf8PathBuf,
}

impl RunDir {
	async fn new() -> Result<RunDir> {
		let tempdir = tempfile::Builder::new()
			.prefix("tangram-vm-")
			.rand_bytes(16)
			.tempdir()
			.context("could not create temporary directory for VM files")?;

		let path: Utf8PathBuf = tempdir
			.path()
			.to_owned()
			.try_into()
			.context("tempdir did not have utf-8 pathname")?;

		// Make the `socket` subdirectory
		tokio::fs::create_dir(path.join("socket"))
			.await
			.context("failed to create 'socket' subdirectory")?;

		Ok(RunDir {
			_tempdir: tempdir,
			path,
		})
	}

	/// Get the path to the boot image.
	#[must_use]
	pub fn boot_image(&self) -> Utf8PathBuf {
		self.path.join("boot.raw")
	}

	/// Get the path to the cloud-init image
	#[must_use]
	pub fn cloud_init_image(&self) -> Utf8PathBuf {
		self.path.join("cloud-init.raw")
	}

	/// Get the path to the log of the boot console
	#[must_use]
	pub fn console_log(&self) -> Utf8PathBuf {
		self.path.join("console.log")
	}

	/// Get the path to a host Unix socket for forwarding
	#[must_use]
	pub fn forwarded_socket_path(&self, sock: &Socket) -> Utf8PathBuf {
		self.path.join("socket").join(&sock.name)
	}

	/// Get the path to the resource dir
	#[must_use]
	pub fn path(&self) -> &Utf8Path {
		self.path.as_ref()
	}
}

/// Mount a disk image into the guest
#[derive(Debug, Clone)]
pub struct Disk {
	/// Path to the raw-format disk image on the host.
	pub image: Utf8PathBuf,

	/// Identifier for the disk (linked in `/dev/disk/by-id/virtio-ID`)
	pub id: String,

	/// Whether the disk is read-only
	pub access: Writability,

	/// Optionally configure an `/etc/fstab` entry for this disk using the given filesystem.
	///
	/// Note: we will set `x-systemd.makefs` and `x-systemd.growfs` on the mount, so the
	/// filesystem will be created if the image is unformatted.
	pub mount: Option<DiskMount>,
}

#[derive(Debug, Clone)]
pub struct DiskMount {
	pub fs: DiskFs,
	pub mountpoint: Utf8PathBuf,
}

#[derive(Debug, Clone, Display, PartialEq, Eq)]
pub enum DiskFs {
	#[display(fmt = "ext4")]
	Ext4,
	#[display(fmt = "btrfs")]
	Btrfs,
	#[display(fmt = "xfs")]
	Xfs,
}

/// Mount a host directory into the guest
#[derive(Debug, Clone)]
pub struct Share {
	/// A unique identifier for this particular share.
	pub tag: String,
	pub host_path: Utf8PathBuf,
	pub guest_path: Utf8PathBuf,
	pub access: Writability,
}

/// Forward unix socket connections to the guest
#[derive(Debug, Clone)]
pub struct Socket {
	/// Name of the socket, for identification to the host.
	pub name: String,

	/// Path to a Unix socket on the guest.
	pub guest_path: Utf8PathBuf,
}

/// Add a user to the guest
#[derive(Debug, Clone)]
pub struct User {
	/// User ID to use. If None, cloud-init picks the next available value in the guest.
	pub uid: Option<u32>,

	/// Username
	pub name: String,

	/// The user's primary group
	pub primary_group: Option<String>,

	/// The groups a user is a member of (by name)
	pub groups: Vec<String>,

	/// User password. If None, password login is disabled.
	pub password: Option<String>,

	/// Enables passwordless sudo
	pub sudo: bool,

	/// Authorized SSH keys for this user
	pub ssh_authorized_keys: Vec<String>,
}

impl User {
	/// Create a [`User`] based on an existing user on the host.
	pub async fn from_host_user(name: &str) -> Result<User> {
		use users::os::unix::UserExt;

		let u = users::get_user_by_name(&name)
			.context(format!("no user named '{name}' exists on the host"))?;

		// Grab the details of their primary group.
		let primary_gid = u.primary_group_id();
		let primary_group = users::get_group_by_gid(primary_gid)
			.context("user's primary group (id {primary_gid}) does not exist")?;
		let primary_group_name = primary_group
			.name()
			.to_str()
			.context("name of primary group (id {primary_gid}) is not Unicode")?
			.to_owned();

		// Grab the details of all their other groups.
		let groups =
			users::get_user_groups(&name, primary_gid).context("error looking up user's groups")?;
		let group_names = groups
			.into_iter()
			.map(|g| {
				g.name()
					.to_str()
					.map(std::borrow::ToOwned::to_owned)
					.context("group name is not Unicode")
			})
			.collect::<Result<Vec<String>>>()?;

		// Read their SSH id_rsa.pub if they have one, and add it to the guest user.
		let ssh_authorized_keys = match Self::read_user_ssh_public_key(u.home_dir()).await {
			Ok(key) => vec![key],
			Err(e) => {
				debug!(user=?name, home=?u.home_dir(), err=?e, "Failed to load ssh public key");
				vec![]
			},
		};

		Ok(User {
			name: name.to_owned(),
			sudo: true,     // everybody gets root inside the vm
			password: None, // we can't determine a user's password.
			uid: Some(u.uid()),
			primary_group: Some(primary_group_name),
			groups: group_names,
			ssh_authorized_keys,
		})
	}

	pub async fn read_user_ssh_public_key(home: &Path) -> Result<String> {
		let path = home.join(".ssh").join("id_rsa.pub");
		let buf = tokio::fs::read(&path)
			.await
			.context("failed to read SSH key file")?;
		let key = String::from_utf8(buf).context("SSH public key contained invalid UTF-8")?;
		Ok(key)
	}

	/// Create a [`User`] based on the current user on the host.
	pub async fn from_current_host_user() -> Result<User> {
		let current_user_name = users::get_current_username()
			.context("the current user has no name")?
			.to_str()
			.context("the current user name is not Unicode")?
			.to_owned();

		User::from_host_user(&current_user_name).await
	}
}

/// Configure a user group in the guest
#[derive(Debug, Clone)]
pub struct Group {
	/// The group name.
	pub name: String,

	/// The group ID to use. If None, cloud-init will pick one.
	pub gid: Option<u32>,
}
