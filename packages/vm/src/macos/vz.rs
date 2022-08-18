//! Create and manage VMs with Apple `Virtualization.framework`

use crate::macos::foundation::{
	DispatchQueue, Id, NSArray, NSError, NSFileHandle, NSString, StrongPtr, BOOL, NSURL, YES,
};
use crate::Writability;
use anyhow::{anyhow, ensure, Context, Result};
use block::ConcreteBlock;
use camino::Utf8Path;
use derive_more::{Deref, From};
use lazy_static::lazy_static;
use objc::runtime::{Class, Object, Sel};
use objc::{class, declare::ClassDecl, msg_send, sel, sel_impl};
use std::borrow::Cow;
use std::ops::Deref;
use std::os::raw::c_void;
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::sync::watch;
use tokio::time::timeout;
use ubyte::ByteUnit;

use super::foundation::NIL;

// Link `Virtualization.framework` when compiling.
#[link(name = "Virtualization", kind = "framework")]
extern "C" {}

/// Implement `std::fmt::Debug` by messaging an [`objc::runtime::Object`]'s `description` selector.
macro_rules! impl_standard_objc_traits {
	($type:ty) => {
		// The wrappers in this file must be safe to move between threads.
		unsafe impl Send for $type {}

		impl ::std::fmt::Debug for $type {
			fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
				let desc = $crate::macos::foundation::NSString::describe(**self);
				write!(f, "{}", desc.as_str())
			}
		}
	};
}

macro_rules! impl_default_with_new {
	($type:ty) => {
		impl ::std::default::Default for $type {
			fn default() -> Self {
				Self::new()
			}
		}
	};
}

/// Implement Objective-C downcasting with `TryFrom`
macro_rules! impl_downcast {
	($any:ty, $result:ty, $class:literal) => {
		impl TryFrom<$any> for $result {
			type Error = anyhow::Error;
			fn try_from(any: $any) -> Result<$result> {
				let cls = Class::get($class).unwrap();
				let is_member: bool = unsafe { msg_send![*any, isKindOfClass: cls] };
				if is_member {
					Ok(Self(any.0))
				} else {
					Err(anyhow!("{any:?} is not kind of class `{}`", $class))
				}
			}
		}
	};
}

// Boot loader implementations
pub trait VZBootLoader: Deref<Target = Id> {}

// VM device configuration
pub trait VZEntropyDeviceConfiguration: Deref<Target = Id> {}
pub trait VZNetworkDeviceConfiguration: Deref<Target = Id> {}
pub trait VZSerialPortConfiguration: Deref<Target = Id> {}
pub trait VZSocketDeviceConfiguration: Deref<Target = Id> {}
pub trait VZStorageDeviceConfiguration: Deref<Target = Id> {}
pub trait VZMemoryBalloonDeviceConfiguration: Deref<Target = Id> {}
pub trait VZDirectorySharingDeviceConfiguration: Deref<Target = Id> {}

// Host device attachments
pub trait VZNetworkDeviceAttachment: Deref<Target = Id> {}
pub trait VZStorageDeviceAttachment: Deref<Target = Id> {}
pub trait VZSerialPortAttachment: Deref<Target = Id> {}

// VM devices created by VZVirtualMachine
pub trait VZSocketDevice: Deref<Target = Id> {}
pub trait VZMemoryBalloonDevice: Deref<Target = Id> {}
pub trait VZDirectorySharingDevice: Deref<Target = Id> {}

// Shared folders
pub trait VZDirectoryShare: Deref<Target = Id> {}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZLinuxBootLoader(StrongPtr);
impl_standard_objc_traits!(VZLinuxBootLoader);
impl VZBootLoader for VZLinuxBootLoader {}

impl VZLinuxBootLoader {
	pub fn with_kernel(kernel: impl AsRef<Utf8Path>) -> VZLinuxBootLoader {
		unsafe {
			let bootloader: Id = msg_send![class!(VZLinuxBootLoader), new];

			// Configure the URL to the kernel image
			let kernel_url = NSURL::file_url_with_path(kernel.as_ref().as_str(), false);
			let _: Id = msg_send![bootloader, initWithKernelURL: *kernel_url];

			VZLinuxBootLoader(StrongPtr::new(bootloader))
		}
	}

	pub fn set_command_line(&mut self, command_line: &str) -> &mut Self {
		unsafe {
			let command_line = NSString::new(command_line);
			let _: Id = msg_send![**self, setCommandLine: *command_line];
		}
		self
	}

	pub fn set_initial_ramdisk(&mut self, initrd: impl AsRef<Utf8Path>) -> &mut Self {
		unsafe {
			// Configure the URL to the kernel image
			let initrd_url = NSURL::file_url_with_path(initrd.as_ref().as_str(), false);
			let _: Id = msg_send![**self, setInitialRamdiskURL: *initrd_url];
		}
		self
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZFileHandleSerialPortAttachment(StrongPtr);
impl_standard_objc_traits!(VZFileHandleSerialPortAttachment);
impl VZSerialPortAttachment for VZFileHandleSerialPortAttachment {}

impl VZFileHandleSerialPortAttachment {
	/// Construct an attachment pointing to stdin and stdout.
	#[must_use]
	pub fn from_stdio() -> Self {
		let stdin = NSFileHandle::from_stdin();
		let stdout = NSFileHandle::from_stdout();
		Self::from_io_handles(&stdin, &stdout)
	}

	/// Construct an attachment pointing to a single file descriptor for writing, but with reads
	/// disabled.
	#[must_use]
	pub fn from_write_handle(write_handle: &NSFileHandle) -> Self {
		unsafe {
			let attachment: Id = msg_send![class!(VZFileHandleSerialPortAttachment), new];
			let _: Id = msg_send![
				attachment,
				initWithFileHandleForReading:NIL
				fileHandleForWriting:**write_handle
			];
			Self(StrongPtr::new(attachment))
		}
	}

	/// Construct an attachment pointing to open file descriptors.
	#[must_use]
	pub fn from_io_handles(read_handle: &NSFileHandle, write_handle: &NSFileHandle) -> Self {
		unsafe {
			let attachment: Id = msg_send![class!(VZFileHandleSerialPortAttachment), new];
			let _: Id = msg_send![
				attachment,
				initWithFileHandleForReading:**read_handle
				fileHandleForWriting:**write_handle
			];
			Self(StrongPtr::new(attachment))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZFileSerialPortAttachment(StrongPtr);
impl_standard_objc_traits!(VZFileSerialPortAttachment);
impl VZSerialPortAttachment for VZFileSerialPortAttachment {}

impl VZFileSerialPortAttachment {
	pub fn new_truncate(path: &Utf8Path) -> Result<VZFileSerialPortAttachment> {
		Self::new(path, false)
	}

	pub fn new_append(path: &Utf8Path) -> Result<VZFileSerialPortAttachment> {
		Self::new(path, true)
	}

	fn new(path: &Utf8Path, append: bool) -> Result<VZFileSerialPortAttachment> {
		unsafe {
			let url = NSURL::file_url_with_path(path.as_str(), false);
			let attachment: Id = msg_send![class!(VZFileSerialPortAttachment), new];

			let err = NSError::nil(); // modified through `error` out-parameter
			let _: Id = msg_send![attachment, initWithURL:*url append:append error:&*err];
			NSError::result_from_nullable(*err)?;

			Ok(Self(StrongPtr::new(attachment)))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioConsoleDeviceSerialPortConfiguration(StrongPtr);
impl_standard_objc_traits!(VZVirtioConsoleDeviceSerialPortConfiguration);
impl VZSerialPortConfiguration for VZVirtioConsoleDeviceSerialPortConfiguration {}

impl VZVirtioConsoleDeviceSerialPortConfiguration {
	/// Create a [`VZVirtioConsoleDeviceSerialPortConfiguration`] with a given IO attachment.
	pub fn with_attachment(attachment: &impl VZSerialPortAttachment) -> Self {
		unsafe {
			let conf: Id = msg_send![class!(VZVirtioConsoleDeviceSerialPortConfiguration), new];
			let _: Id = msg_send![conf, setAttachment: **attachment];
			Self(StrongPtr::new(conf))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZDiskImageStorageDeviceAttachment(StrongPtr);
impl_standard_objc_traits!(VZDiskImageStorageDeviceAttachment);
impl VZStorageDeviceAttachment for VZDiskImageStorageDeviceAttachment {}

impl VZDiskImageStorageDeviceAttachment {
	pub fn new(path: impl AsRef<Utf8Path>, access: Writability) -> Result<Self> {
		unsafe {
			let url = NSURL::file_url_with_path(path.as_ref().as_str(), false);
			let attachment: Id = msg_send![class!(VZDiskImageStorageDeviceAttachment), new];

			let err = NSError::nil(); // modified through `error` out-parameter
			let read_only = access == Writability::ReadOnly;
			let _: Id = msg_send![attachment, initWithURL:*url readOnly:read_only error:&*err];
			NSError::result_from_nullable(*err)?;

			Ok(Self(StrongPtr::new(attachment)))
		}
	}

	pub fn from_cache_config(
		path: impl AsRef<Utf8Path>,
		access: Writability,
		cache_mode: VZDiskImageCachingMode,
		sync_mode: VZDiskImageSynchronizationMode,
	) -> Result<Self> {
		unsafe {
			let url = NSURL::file_url_with_path(path.as_ref().as_str(), false);
			let attachment: Id = msg_send![class!(VZDiskImageStorageDeviceAttachment), new];

			let err = NSError::nil(); // modified through `error` out-parameter
			let read_only = access == Writability::ReadOnly;
			let cache_mode = cache_mode as u64;
			let sync_mode = sync_mode as u64;
			let _: Id = msg_send![
				attachment,
				initWithURL:*url
				readOnly:read_only
				cachingMode:cache_mode
				synchronizationMode:sync_mode
				error:&*err
			];
			NSError::result_from_nullable(*err)?;

			Ok(Self(StrongPtr::new(attachment)))
		}
	}
}

pub enum VZDiskImageCachingMode {
	Automatic = 0,
	Cached = 2,
	Uncached = 1,
}

pub enum VZDiskImageSynchronizationMode {
	Full = 1,
	Fsync = 2,
	None = 3,
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioBlockDeviceConfiguration(StrongPtr);
impl_standard_objc_traits!(VZVirtioBlockDeviceConfiguration);
impl VZStorageDeviceConfiguration for VZVirtioBlockDeviceConfiguration {}

impl VZVirtioBlockDeviceConfiguration {
	pub fn with_attachment(attachment: &impl VZStorageDeviceAttachment) -> Self {
		unsafe {
			let block_device: Id = msg_send![class!(VZVirtioBlockDeviceConfiguration), new];
			let _: Id = msg_send![block_device, initWithAttachment: **attachment];
			Self(StrongPtr::new(block_device))
		}
	}

	pub fn set_identifier(&mut self, identifier: &str) -> Result<&mut Self> {
		// Validate the identifier
		ensure!(
			identifier
				.chars()
				.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
			"invalid identifier {:?}: contains bad characters",
			identifier
		);
		ensure!(
			identifier.len() <= 20,
			"invalid identifier {:?}: longer than 20 characters",
			identifier
		);

		let _: Id =
			unsafe { msg_send![**self, setBlockDeviceIdentifier: *NSString::new(identifier)] };

		Ok(self)
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioNetworkDeviceConfiguration(StrongPtr);
impl VZNetworkDeviceConfiguration for VZVirtioNetworkDeviceConfiguration {}
impl_standard_objc_traits!(VZVirtioNetworkDeviceConfiguration);

impl VZVirtioNetworkDeviceConfiguration {
	pub fn with_attachment(attachment: &impl VZNetworkDeviceAttachment) -> Self {
		unsafe {
			let config: Id = msg_send![class!(VZVirtioNetworkDeviceConfiguration), new];
			let _: Id = msg_send![config, setAttachment: **attachment];
			Self(StrongPtr::new(config))
		}
	}

	pub fn set_mac_address(&mut self, mac: &VZMACAddress) -> &mut Self {
		unsafe {
			let _: Id = msg_send![**self, setMACAddress:**mac];
		}
		self
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZMACAddress(StrongPtr);

impl VZMACAddress {
	#[must_use]
	pub fn random_local_address() -> Self {
		unsafe {
			let mac: Id = msg_send![class!(VZMACAddress), randomLocallyAdministeredAddress];
			Self(StrongPtr::new(mac))
		}
	}

	fn as_string(&self) -> String {
		let str_ptr = unsafe { StrongPtr::new(msg_send![**self, string]) };
		let ns_string = NSString::from(str_ptr);
		ns_string.as_str().to_string()
	}
}

impl std::fmt::Debug for VZMACAddress {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "VZMACAddress({:?})", self.as_string())
	}
}

impl std::str::FromStr for VZMACAddress {
	type Err = anyhow::Error;
	fn from_str(string: &str) -> Result<Self> {
		let ns_string = NSString::new(string);
		let mac_ptr: StrongPtr = unsafe {
			let mac: Id = msg_send![class!(VZMACAddress), new];
			let mac: Id = msg_send![mac, initWithString:*ns_string];
			StrongPtr::new(mac)
		};

		if mac_ptr.is_null() {
			Err(anyhow!("invalid MAC address string: {}", string))
		} else {
			Ok(Self(mac_ptr))
		}
	}
}

impl From<macaddr::MacAddr6> for VZMACAddress {
	fn from(mac: macaddr::MacAddr6) -> VZMACAddress {
		let string = mac.to_string();
		string
			.parse()
			.expect("macaddr::MacAddr6 returned wrong format")
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZNATNetworkDeviceAttachment(StrongPtr);
impl VZNetworkDeviceAttachment for VZNATNetworkDeviceAttachment {}
impl_standard_objc_traits!(VZNATNetworkDeviceAttachment);
impl_default_with_new!(VZNATNetworkDeviceAttachment);

impl VZNATNetworkDeviceAttachment {
	#[must_use]
	pub fn new() -> Self {
		unsafe {
			let attachment: Id = msg_send![class!(VZNATNetworkDeviceAttachment), new];
			Self(StrongPtr::new(attachment))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioEntropyDeviceConfiguration(StrongPtr);
impl VZEntropyDeviceConfiguration for VZVirtioEntropyDeviceConfiguration {}
impl_standard_objc_traits!(VZVirtioEntropyDeviceConfiguration);
impl_default_with_new!(VZVirtioEntropyDeviceConfiguration);

impl VZVirtioEntropyDeviceConfiguration {
	#[must_use]
	pub fn new() -> Self {
		unsafe {
			let ent: Id = msg_send![class!(VZVirtioEntropyDeviceConfiguration), new];
			Self(StrongPtr::new(ent))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioSocketDeviceConfiguration(StrongPtr);
impl VZSocketDeviceConfiguration for VZVirtioSocketDeviceConfiguration {}
impl_standard_objc_traits!(VZVirtioSocketDeviceConfiguration);
impl_default_with_new!(VZVirtioSocketDeviceConfiguration);

impl VZVirtioSocketDeviceConfiguration {
	#[must_use]
	pub fn new() -> Self {
		unsafe {
			let conf: Id = msg_send![class!(VZVirtioSocketDeviceConfiguration), new];
			Self(StrongPtr::new(conf))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct AnyVZSocketDevice(StrongPtr);
impl VZSocketDevice for AnyVZSocketDevice {}
impl_standard_objc_traits!(AnyVZSocketDevice);

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioSocketDevice(StrongPtr);
impl VZSocketDevice for VZVirtioSocketDevice {}
impl_standard_objc_traits!(VZVirtioSocketDevice);
impl_downcast!(
	AnyVZSocketDevice,
	VZVirtioSocketDevice,
	"VZVirtioSocketDevice"
);
unsafe impl Sync for VZVirtioSocketDevice {} // TODO: not sure this is actually safe

impl VZVirtioSocketDevice {
	/// Connect to a vsock port.
	///
	/// Note: you must pass the same dispatch queue used for all other VM operations.
	pub async fn connect_to_port(
		&self,
		queue: &DispatchQueue,
		port: u32,
	) -> Result<VZVirtioSocketConnection> {
		let establish_connection = queue.promise(move |promise| {
			let callback = ConcreteBlock::new(move |conn: Id, err: Id| {
				if err.is_null() {
					promise.resolve(Ok(unsafe {
						VZVirtioSocketConnection::from(StrongPtr::retain(conn))
					}));
				} else {
					promise.resolve(Err(unsafe { NSError::from(StrongPtr::retain(err)) }));
				}
			})
			.copy();

			// Connect to the VM port
			let _: Id =
				unsafe { msg_send![**self, connectToPort:port completionHandler:&*callback] };
		});

		// NOTE: This is a load-bearing timeout.
		//
		// Occasionally, Virtualization.framework will forget to call the
		// completion handler. This timeout will make sure we don't
		// hang in that case.
		//
		// Generally, this will happen if you make many calls to `connect_to_port`
		// in rapid succession (waiting, e.g. 10ms in between).
		let result = timeout(Duration::from_millis(300), establish_connection)
			.await
			.context("timed out while connecting to vsock")?
			.context("VM dropped connection callback without calling it first")?;

		result.map_err(Into::into)
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioSocketConnection(StrongPtr);
impl_standard_objc_traits!(VZVirtioSocketConnection);

impl VZVirtioSocketConnection {
	#[must_use]
	pub fn source_port(&self) -> u32 {
		unsafe { msg_send![**self, sourcePort] }
	}

	#[must_use]
	pub fn destination_port(&self) -> u32 {
		unsafe { msg_send![**self, destinationPort] }
	}

	/// Gets the file descriptor used to send data through this connection.
	/// If the connection is closed, this function returns `None`.
	///
	/// Note: When the [`VZVirtioSocketConnection`] is dropped, this file descriptor will be
	/// closed.
	#[must_use]
	fn file_descriptor(&self) -> Option<RawFd> {
		let fd: i32 = unsafe { msg_send![**self, fileDescriptor] };
		if let Ok(positive_fd) = <i32 as TryInto<u32>>::try_into(fd) {
			let raw_fd: RawFd = positive_fd
				.try_into()
				.expect("cannot convert u32 to file descriptor");
			Some(raw_fd)
		} else {
			// Virtualization.framework uses `-1` to refer to a closed or invalid fd.
			None
		}
	}

	/// Wrap [`VZVirtioSocketConnection::file_descriptor`] with a [`tokio::net::UnixStream`],
	/// so we can do I/O through it.
	fn connection_stream(&self) -> Result<tokio::net::UnixStream> {
		use std::os::unix::io::FromRawFd;
		let fd = self
			.file_descriptor()
			.context("Not connected, no file descriptor to use")?;
		let std_stream = unsafe { std::os::unix::net::UnixStream::from_raw_fd(fd) };
		let tokio_stream = tokio::net::UnixStream::from_std(std_stream)
			.context("failed to create tokio UnixStream from std")?;
		Ok(tokio_stream)
	}

	/// Create a [`VsockStream`] from the [`VZVirtioSocketConnection`], which implements
	/// `Read`/`Write`/`AsyncRead`/`AsyncWrite`.
	///
	/// This method takes `&mut self` because `Virtualization.framework` only gives us a single
	/// file descriptor, which is unsafe to use in multiple `UnixStream` objects (as they might be used
	/// concurrently).
	///
	/// The stream is bound to the lifetime of the `VZVirtioSocketConnection` because
	/// `Virtualization.framework` will close the file descriptor on dealloc.
	pub fn stream(&mut self) -> Result<VsockStream<'_>> {
		Ok(VsockStream {
			stream: self.connection_stream()?,
			_conn: Cow::Borrowed(self),
		})
	}

	/// Convert the [`VZVirtioSocketConnection`] into a [`VsockStream`], which implements
	/// `Read`/`Write`/`AsyncRead`/`AsyncWrite`.
	///
	/// This method consumes `self`.
	pub fn into_stream(self) -> Result<VsockStream<'static>> {
		Ok(VsockStream {
			stream: self.connection_stream()?,
			_conn: Cow::Owned(self),
		})
	}
}

/// A `VSock` IO stream.
///
/// Constructed by [`VZVirtioSocketConnection::stream`]
pub struct VsockStream<'a> {
	// Ensure the VZVirtioSocketConnection is alive.
	// We need this, because as soon as it reaches `dealloc`, the file descriptor is closed.
	//
	// Don't access or modify this field, except to drop the struct---Apple's documentation gives
	// zero indication as to whether a VZVirtioSocketConnection is threadsafe.
	_conn: Cow<'a, VZVirtioSocketConnection>,

	// NOTE: This is a bit of a hack, because the FD we get from Virtualization.framework is not
	// actually the FD of a Unix stream per se---it's of an AF_VSOCK socket. So we must be
	// careful with what we use the file for.
	stream: tokio::net::UnixStream,
}

// The one non-Sync member of this struct is the VZVirtioSocketConnection.
// We don't ever access the VZVirtioSocketConnection itself, we only hold onto it
// to prove it has not been deallocated.
unsafe impl<'a> Send for VsockStream<'a> {}
unsafe impl<'a> Sync for VsockStream<'a> {}

impl<'f> AsyncRead for VsockStream<'f> {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context,
		buf: &mut ReadBuf,
	) -> Poll<std::io::Result<()>> {
		Pin::new(&mut self.stream).poll_read(cx, buf)
	}
}

impl<'f> AsyncWrite for VsockStream<'f> {
	fn poll_write(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context,
		buf: &[u8],
	) -> Poll<std::io::Result<usize>> {
		Pin::new(&mut self.stream).poll_write(cx, buf)
	}

	fn poll_flush(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		Pin::new(&mut self.stream).poll_flush(cx)
	}

	fn poll_shutdown(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), std::io::Error>> {
		Pin::new(&mut self.stream).poll_shutdown(cx)
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioTraditionalMemoryBalloonDeviceConfiguration(StrongPtr);
impl VZMemoryBalloonDeviceConfiguration for VZVirtioTraditionalMemoryBalloonDeviceConfiguration {}
impl_standard_objc_traits!(VZVirtioTraditionalMemoryBalloonDeviceConfiguration);
impl_default_with_new!(VZVirtioTraditionalMemoryBalloonDeviceConfiguration);

impl VZVirtioTraditionalMemoryBalloonDeviceConfiguration {
	#[must_use]
	pub fn new() -> VZVirtioTraditionalMemoryBalloonDeviceConfiguration {
		unsafe {
			let dev: Id = msg_send![
				class!(VZVirtioTraditionalMemoryBalloonDeviceConfiguration),
				new
			];
			Self(StrongPtr::new(dev))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct AnyVZMemoryBalloonDevice(StrongPtr);
impl VZMemoryBalloonDevice for AnyVZMemoryBalloonDevice {}
impl_standard_objc_traits!(AnyVZMemoryBalloonDevice);

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioTraditionalMemoryBalloonDevice(StrongPtr);
impl VZMemoryBalloonDevice for VZVirtioTraditionalMemoryBalloonDevice {}
impl_standard_objc_traits!(VZVirtioTraditionalMemoryBalloonDevice);
impl_downcast!(
	AnyVZMemoryBalloonDevice,
	VZVirtioTraditionalMemoryBalloonDevice,
	"VZVirtioTraditionalMemoryBalloonDevice"
);

impl VZVirtioTraditionalMemoryBalloonDevice {
	/// Get the current target guest memory size from the balloon device.
	#[must_use]
	pub fn target_memory_size(&self) -> ByteUnit {
		let bytes: u64 = unsafe { msg_send![**self, targetVirtualMachineMemorySize] };
		ByteUnit::Byte(bytes)
	}

	/// Set the current target guest memory size, informing the balloon device.
	pub fn set_target_memory_size(&self, size: ByteUnit) {
		let bytes = size.as_u64();
		unsafe {
			let _: Id = msg_send![**self, setTargetVirtualMachineMemorySize: bytes];
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZSharedDirectory(StrongPtr);
impl_standard_objc_traits!(VZSharedDirectory);

impl VZSharedDirectory {
	#[must_use]
	pub fn new(host_path: &Utf8Path, writability: Writability) -> VZSharedDirectory {
		unsafe {
			let dir: Id = msg_send![class!(VZSharedDirectory), alloc];

			let url = NSURL::file_url_with_path(host_path.as_str(), false);
			let read_only = writability == Writability::ReadOnly;
			let _: Id = msg_send![dir, initWithURL:url readOnly:read_only];

			VZSharedDirectory(StrongPtr::new(dir))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZSingleDirectoryShare(StrongPtr);
impl VZDirectoryShare for VZSingleDirectoryShare {}
impl_standard_objc_traits!(VZSingleDirectoryShare);

impl VZSingleDirectoryShare {
	#[must_use]
	pub fn new(shared_dir: &VZSharedDirectory) -> VZSingleDirectoryShare {
		unsafe {
			let share: Id = msg_send![class!(VZSingleDirectoryShare), new];
			let _: Id = msg_send![share, initWithDirectory: **shared_dir];
			VZSingleDirectoryShare(StrongPtr::new(share))
		}
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtioFileSystemDeviceConfiguration(StrongPtr);
impl VZDirectorySharingDeviceConfiguration for VZVirtioFileSystemDeviceConfiguration {}
impl_standard_objc_traits!(VZVirtioFileSystemDeviceConfiguration);

impl VZVirtioFileSystemDeviceConfiguration {
	pub fn new(tag: &str) -> Result<VZVirtioFileSystemDeviceConfiguration> {
		// Make sure the tag is valid
		let tag = Self::validate_tag(tag)?;

		unsafe {
			let conf: Id = msg_send![class!(VZVirtioFileSystemDeviceConfiguration), new];
			let _: Id = msg_send![conf, initWithTag:*tag];
			Ok(Self(StrongPtr::new(conf)))
		}
	}

	pub fn set_share(&mut self, share: &dyn VZDirectoryShare) {
		unsafe {
			let _: Id = msg_send![**self, setShare:**share];
		}
	}

	pub fn validate_tag(tag: &str) -> Result<NSString> {
		let err = NSError::nil();
		let tag = NSString::new(tag);
		unsafe {
			let _: Id = msg_send![class!(VZVirtioFileSystemDeviceConfiguration), validateTag:*tag error: &*err];
		}
		NSError::result_from_nullable(*err)?;
		Ok(tag)
	}
}

#[derive(Clone, Deref, From)]
#[deref(forward)]
pub struct VZVirtualMachineConfiguration(StrongPtr);
impl_standard_objc_traits!(VZVirtualMachineConfiguration);
impl_default_with_new!(VZVirtualMachineConfiguration);

impl VZVirtualMachineConfiguration {
	#[must_use]
	pub fn new() -> VZVirtualMachineConfiguration {
		unsafe {
			let conf: Id = msg_send![class!(VZVirtualMachineConfiguration), new];
			Self(StrongPtr::new(conf))
		}
	}

	pub fn set_boot_loader(&mut self, boot_loader: &impl VZBootLoader) -> &mut Self {
		let _: Id = unsafe { msg_send![**self, setBootLoader: **boot_loader] };
		self
	}

	pub fn set_cpu_count(&mut self, cpu_count: usize) -> &mut Self {
		let _: Id = unsafe { msg_send![**self, setCPUCount: cpu_count] };
		self
	}

	pub fn set_memory_size(&mut self, memory_size: ByteUnit) -> &mut Self {
		// Round memory size down to a multiple of one megabyte.
		// Anything that isn't a multiple of one megabyte causes a VM configuration error.
		let rounded_down = ByteUnit::Mebibyte((memory_size / ByteUnit::Mebibyte(1)).as_u64());

		let _: Id = unsafe { msg_send![**self, setMemorySize: rounded_down.as_u64()] };
		self
	}

	pub fn set_serial_ports(
		&mut self,
		serial_ports: &[&dyn VZSerialPortConfiguration],
	) -> &mut Self {
		let serial_ports = NSArray::from_deref(serial_ports);
		let _: Id = unsafe { msg_send![**self, setSerialPorts: *serial_ports] };
		self
	}

	pub fn set_socket_devices(
		&mut self,
		socket_devices: &[&dyn VZSocketDeviceConfiguration],
	) -> &mut Self {
		let socket_devices = NSArray::from_deref(socket_devices);
		let _: Id = unsafe { msg_send![**self, setSocketDevices: *socket_devices] };
		self
	}

	pub fn set_entropy_devices(
		&mut self,
		entropy_devices: &[&dyn VZEntropyDeviceConfiguration],
	) -> &mut Self {
		let entropy_devices = NSArray::from_deref(entropy_devices);
		let _: Id = unsafe { msg_send![**self, setEntropyDevices: *entropy_devices] };
		self
	}

	pub fn set_storage_devices(
		&mut self,
		serial_ports: &[&dyn VZStorageDeviceConfiguration],
	) -> &mut Self {
		let storage_devices = NSArray::from_deref(serial_ports);
		let _: Id = unsafe { msg_send![**self, setStorageDevices: *storage_devices] };
		self
	}

	pub fn set_network_devices(
		&mut self,
		network_devices: &[&dyn VZNetworkDeviceConfiguration],
	) -> &mut Self {
		let network_devices = NSArray::from_deref(network_devices);
		let _: Id = unsafe { msg_send![**self, setNetworkDevices: *network_devices] };
		self
	}

	pub fn set_balloon_device(
		&mut self,
		balloon_device: &dyn VZMemoryBalloonDeviceConfiguration,
	) -> &mut Self {
		let balloon_devices = NSArray::from_deref(&[balloon_device]);
		let _: Id = unsafe { msg_send![**self, setMemoryBalloonDevices: *balloon_devices] };
		self
	}

	pub fn set_directory_sharing_devices(
		&mut self,
		directory_sharing_devices: &[&dyn VZDirectorySharingDeviceConfiguration],
	) -> &mut Self {
		let directory_sharing_devices = NSArray::from_deref(directory_sharing_devices);
		let _: Id =
			unsafe { msg_send![**self, setDirectorySharingDevices: *directory_sharing_devices] };
		self
	}

	pub fn validate(&self) -> Result<()> {
		unsafe {
			let err = NSError::nil(); // modified through `error` out-parameter
			let _: Id = msg_send![**self, validateWithError:&*err];
			if err.is_null() {
				Ok(())
			} else {
				Err(err.into())
			}
		}
	}
}

#[derive(From)]
pub struct VZVirtualMachine {
	vm: StrongPtr,
	dispatch_queue: DispatchQueue,

	// SAFETY: `Arc` has the effect of pinning the `VmDelegateHandles`.
	// (there is no way to get a `&mut` reference, so the `VmDelegateHandles` cannot be moved)
	delegate_handles: Arc<VmDelegateHandles>,
	_delegate: StrongPtr,
}
impl_standard_objc_traits!(VZVirtualMachine);

unsafe impl Sync for VZVirtualMachine {}

struct VmDelegateHandles {
	/// Channel of stop events: either `Ok(())` if the guest stopped itself, or `Err(NSError)` if
	/// the virtual machine stopped due to an error.
	stop_event: watch::Sender<Result<(), NSError>>,

	// TODO: handle network device attachment errors here.

	// SAFETY: This struct cannot safely move, because the objective-c delegate stores
	// a reference to it as a raw pointer.
	_pinned: std::marker::PhantomPinned,
}

impl std::ops::Deref for VZVirtualMachine {
	type Target = Id;
	fn deref(&self) -> &Id {
		&self.vm
	}
}

impl VZVirtualMachine {
	pub fn new(config: &VZVirtualMachineConfiguration) -> Result<VZVirtualMachine> {
		// Make sure the delegate class has been declared and registered with the Objective-C
		// runtime.
		lazy_static! {
			static ref VM_DELEGATE_CLASS: &'static Class = VZVirtualMachine::declare_delegate();
		}

		// Validate the VM config. If it's invalid, we can't boot.
		config.validate().context("invalid VM configuration")?;

		// Create a dispatch queue to run VM operations.
		// Note: According to Apple's docs, this *must* be a serial dispatch queue.
		let dispatch_queue = DispatchQueue::new_serial("tangram_vm::apple_vz::VZVirtualMachine");

		// Create an instance of the delegate handles.
		let delegate_handles = Arc::new(VmDelegateHandles {
			stop_event: watch::channel(Ok(())).0,
			_pinned: std::marker::PhantomPinned::default(),
		});

		// Create a delegate object
		let delegate: StrongPtr = unsafe {
			let id: Id = msg_send![*VM_DELEGATE_CLASS, new];

			let handles = Arc::clone(&delegate_handles);
			let handles_ptr: *const VmDelegateHandles = Arc::into_raw(handles);
			(*id).set_ivar("_handles", handles_ptr.cast::<c_void>());

			StrongPtr::new(id)
		};

		// Create the VM object
		let vm = unsafe {
			let vm: Id = msg_send![class!(VZVirtualMachine), new];
			let _: Id = msg_send![vm, initWithConfiguration:**config queue:*dispatch_queue];

			// Set the VM's (weak) reference to the delegate.
			// We still retain ownership of the delegate StrongPtr, so the delegate will
			// be retained until the `VZVirtualMachine` is dropped.
			let _: Id = msg_send![vm, setDelegate: *delegate];

			StrongPtr::new(vm)
		};

		Ok(VZVirtualMachine {
			vm,
			dispatch_queue,
			delegate_handles,
			_delegate: delegate,
		})
	}

	/// Get the dispatch queue used by tasks related to this virtual machine.
	#[must_use]
	pub fn dispatch_queue(&self) -> DispatchQueue {
		self.dispatch_queue.clone() // calls dispatch_retain()
	}

	/// Declare an Objective-C delegate class to receive VM events.
	fn declare_delegate() -> &'static Class {
		/// Get a `&VmDelegateHandles` from a `this`-pointer.
		unsafe fn get_handles(this: &Object) -> &VmDelegateHandles {
			let void_ptr: *const c_void = *this.get_ivar::<*const c_void>("_handles");
			&*void_ptr.cast()
		}

		/// Dealloc method: drop the Arc<VmDelegateHandles>
		extern "C" fn dealloc(this: &Object, _cmd: Sel) {
			unsafe {
				// Reconstruct the Arc<VmDelegateHandles> and drop it.
				let handles_ptr: *const c_void = *this.get_ivar("_handles");
				let handles_ptr: *const VmDelegateHandles = handles_ptr.cast();
				drop(Arc::from_raw(handles_ptr));
			}
		}

		/// Report when the guest stopped of its own volition
		extern "C" fn guest_did_stop(this: &Object, _cmd: Sel, _vm: Id) {
			let handles = unsafe { get_handles(this) };

			// Send and ignore error---we don't care if there are no listeners.
			drop(handles.stop_event.send(Ok(())));
		}

		let mut decl = ClassDecl::new("TGVirtualMachineDelegate", class!(NSObject))
			.expect("failed to create class declaration");

		// Add an instance variable, storing a reference to the VmDelegateHandles.
		decl.add_ivar::<*const c_void>("_handles");

		unsafe {
			decl.add_method(sel!(dealloc), dealloc as extern "C" fn(&Object, Sel));
			decl.add_method(
				sel!(guestDidStopVirtualMachine:),
				guest_did_stop as extern "C" fn(&Object, Sel, Id),
			);
		};

		decl.register()
	}

	/// Start the virtual machine.
	pub async fn start(&self) -> Result<()> {
		let start_err: NSError = self
			.dispatch_queue
			.promise(|promise| {
				let callback = ConcreteBlock::new(move |err: Id| {
					let err = NSError::from(unsafe { StrongPtr::retain(err) });
					promise.resolve(err);
				})
				.copy();

				// Start the VM
				let _: Id = unsafe { msg_send![**self, startWithCompletionHandler: &*callback] };
			})
			.await
			.context("VM dropped callback without starting")?;

		NSError::result_from_nullable(*start_err)?;
		Ok(())
	}

	/// Kill the virtual machine.
	pub async fn kill(&self) -> Result<()> {
		let kill_err: NSError = self
			.dispatch_queue
			.promise(|promise| {
				let callback = ConcreteBlock::new(move |err: Id| {
					let err = NSError::from(unsafe { StrongPtr::retain(err) });
					promise.resolve(err);
				})
				.copy();

				// Kil the VM
				let _: Id = unsafe { msg_send![**self, stopWithCompletionHandler: &*callback] };
			})
			.await
			.context("VM dropped callback without stopping")?;
		NSError::result_from_nullable(*kill_err)?;
		Ok(())
	}

	/// Ask the guest OS to shut down.
	///
	/// Follow a call to [`VZVirtualMachine::request_shutdown`] with a call to
	/// [`VZVirtualMachine::wait`] to wait for the VM to completely power off.
	pub async fn request_shutdown(&self) -> Result<()> {
		let err = self
			.dispatch_queue
			.promise(|promise| {
				let err = NSError::nil();
				let _: Id = unsafe { msg_send![**self, requestStopWithError:&*err] };
				promise.resolve(err);
			})
			.await
			.unwrap();
		NSError::result_from_nullable(*err)?;
		Ok(())
	}

	/// Wait for the guest to power off.
	pub async fn wait(&self) -> Result<()> {
		// Wait for a message on the stop_event channel, sent by the delegate.
		let mut listener = self.delegate_handles.stop_event.subscribe();
		let stop_event = listener.changed().await;

		// Convert NSError to anyhow::Error
		stop_event.map_err(|e| anyhow!(e))
	}

	/// Get the list of socket devices attached to the VM
	#[must_use]
	pub fn socket_devices(&self) -> Vec<AnyVZSocketDevice> {
		let devices: NSArray<AnyVZSocketDevice> = unsafe {
			let ptr = StrongPtr::retain(msg_send![**self, socketDevices]);
			NSArray::from(ptr)
		};
		devices.into_vec()
	}

	/// Get the memory balloon device attached to the VM, if any.
	#[must_use]
	pub fn balloon_device(&self) -> Option<AnyVZMemoryBalloonDevice> {
		let devices: NSArray<AnyVZMemoryBalloonDevice> = unsafe {
			let ptr = StrongPtr::retain(msg_send![**self, memoryBalloonDevices]);
			NSArray::from(ptr)
		};
		match devices.count() {
			0 => None,
			1 => Some(devices.object_at_index(0)),

			// "Important: Create only one
			// VZVirtioTraditionalMemoryBalloonDeviceConfiguration object for your virtual machine"
			// https://developer.apple.com/documentation/virtualization/vzvirtiotraditionalmemoryballoondeviceconfiguration?language=objc
			_ => unreachable!("VZVirtualMachine must not have more than one balloon device"),
		}
	}
}

/// Check whether virtualization is supported on this platform.
#[must_use]
pub fn virtualization_supported() -> bool {
	let b: BOOL = unsafe { msg_send![class!(VZVirtualMachine), isSupported] };
	b == YES
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn apple_vz_supported() {
		assert!(virtualization_supported());
	}
}
