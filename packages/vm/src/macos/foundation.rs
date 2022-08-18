//! Common Objective-C utilities.
//!
//! Adapted from: <https://github.com/suzusuzu/virtualization-rs/blob/main/src/base.rs>
#![allow(dead_code)]

pub use block::{Block, ConcreteBlock};
use derive_more::{Deref, From};
pub use objc::rc::{StrongPtr, WeakPtr};
pub use objc::runtime::{Class, Object, BOOL, NO, YES};
pub use objc::{class, msg_send, sel, sel_impl};
use std::marker::PhantomData;
use std::mem;
use std::os::raw::{c_char, c_void};
use std::os::unix::io::{AsRawFd, OwnedFd};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub type Id = *mut objc::runtime::Object;
pub const NIL: Id = 0 as Id;

#[derive(Deref, From)]
#[deref(forward)]
pub struct NSArray<T> {
	pub p: StrongPtr,
	#[deref(ignore)]
	pub _phantom: PhantomData<T>,
}
unsafe impl<T: Send> Send for NSArray<T> {}

impl<T> From<StrongPtr> for NSArray<T> {
	fn from(ptr: StrongPtr) -> NSArray<T> {
		NSArray {
			p: ptr,
			_phantom: PhantomData,
		}
	}
}

impl<T> NSArray<T> {
	pub fn array_with_objects(objects: &[Id]) -> NSArray<T> {
		unsafe {
			let p = StrongPtr::new(
				msg_send![class!(NSArray), arrayWithObjects:objects.as_ptr() count:objects.len()],
			);
			NSArray {
				p,
				_phantom: PhantomData,
			}
		}
	}

	#[must_use]
	pub fn count(&self) -> usize {
		unsafe { msg_send![*self.p, count] }
	}
}

impl NSArray<Id> {
	/// Create a `NSArray<Id>` by calling the `Deref` impl on a slice of references to structs that
	/// dereference to `Id`.
	pub fn from_deref<O>(objects: &[&O]) -> Self
	where
		O: std::ops::Deref<Target = Id> + ?Sized,
	{
		let ids: Vec<Id> = objects.iter().map(|o| ***o).collect();
		NSArray::<Id>::array_with_objects(&ids)
	}
}

impl<T: From<StrongPtr>> NSArray<T> {
	#[must_use]
	pub fn object_at_index(&self, index: usize) -> T {
		assert!(index < self.count());
		unsafe { T::from(StrongPtr::retain(msg_send![*self.p, objectAtIndex: index])) }
	}

	/// Unwrap the `NSArray` into a `Vec`.
	///
	/// This will send a `retain` to every element of the array, and `release` the container.
	#[must_use]
	pub fn into_vec(self) -> Vec<T> {
		let len = self.count();
		let mut items = Vec::with_capacity(len);
		for i in 0..len {
			items.push(self.object_at_index(i)); // `retain` arr[i]
		}
		drop(self.p); // `release` the container
		items
	}
}

const UTF8_ENCODING: usize = 4;
#[derive(Deref, From)]
#[deref(forward)]
pub struct NSString(StrongPtr);
unsafe impl Send for NSString {}

impl NSString {
	#[must_use]
	pub fn new(string: &str) -> NSString {
		unsafe {
			let alloc: Id = msg_send![class!(NSString), alloc];
			let p = StrongPtr::new(
				msg_send![alloc, initWithBytes:string.as_ptr() length:string.len() encoding:UTF8_ENCODING as Id],
			);
			NSString(p)
		}
	}

	#[must_use]
	#[allow(clippy::not_unsafe_ptr_arg_deref)] // We don't dereference, we message
	pub fn describe(obj: Id) -> NSString {
		unsafe {
			let desc_id: Id = msg_send![obj, description];
			NSString(StrongPtr::new(desc_id))
		}
	}

	#[must_use]
	pub fn len(&self) -> usize {
		unsafe { msg_send![*self.0, lengthOfBytesUsingEncoding: UTF8_ENCODING] }
	}

	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	#[must_use]
	pub fn as_str(&self) -> &str {
		unsafe {
			let bytes = {
				let bytes: *const c_char = msg_send![*self.0, UTF8String];
				bytes.cast::<u8>()
			};
			let len = self.len();
			let bytes = std::slice::from_raw_parts(bytes, len);
			std::str::from_utf8(bytes).unwrap()
		}
	}
}

#[derive(Deref, From)]
#[deref(forward)]
pub struct NSURL(StrongPtr);
unsafe impl Send for NSURL {}

impl NSURL {
	#[must_use]
	pub fn url_with_string(url: &str) -> NSURL {
		unsafe {
			let url_nsstring = NSString::new(url);
			let p = StrongPtr::retain(msg_send![class!(NSURL), URLWithString: url_nsstring]);
			NSURL(p)
		}
	}

	#[must_use]
	pub fn file_url_with_path(path: &str, is_directory: bool) -> NSURL {
		unsafe {
			let path_nsstring = NSString::new(path);
			let is_directory_ = if is_directory { YES } else { NO };
			let p = StrongPtr::retain(
				msg_send![class!(NSURL), fileURLWithPath:path_nsstring isDirectory:is_directory_],
			);
			NSURL(p)
		}
	}

	#[must_use]
	pub fn check_resource_is_reachable_and_return_error(&self) -> bool {
		let b: BOOL = unsafe { msg_send![*self.0, checkResourceIsReachableAndReturnError: NIL] };
		b == YES
	}

	#[must_use]
	pub fn absolute_url(&self) -> NSURL {
		unsafe {
			let p = StrongPtr::retain(msg_send![*self.0, absoluteURL]);
			NSURL(p)
		}
	}
}

#[derive(Deref, From)]
#[deref(forward)]
pub struct NSFileHandle(StrongPtr);
unsafe impl Send for NSFileHandle {}

impl NSFileHandle {
	pub fn from_fd<T>(file: T) -> NSFileHandle
	where
		OwnedFd: From<T>,
	{
		// Forget the OwnedFd, so the file does not get closed.
		// NSFileHandle will close it on dealloc.
		let owned_fd = OwnedFd::from(file);
		let raw_fd = owned_fd.as_raw_fd();
		mem::forget(owned_fd);

		unsafe {
			let handle: Id = msg_send![class!(NSFileHandle), new];
			let _: Id = msg_send![handle, initWithFileDescriptor:raw_fd closeOnDealloc:YES];
			NSFileHandle(StrongPtr::new(handle))
		}
	}

	#[must_use]
	pub fn from_stdin() -> NSFileHandle {
		unsafe {
			let p = StrongPtr::retain(msg_send![class!(NSFileHandle), fileHandleWithStandardInput]);
			NSFileHandle(p)
		}
	}

	#[must_use]
	pub fn from_stdout() -> NSFileHandle {
		unsafe {
			let p = StrongPtr::retain(msg_send![
				class!(NSFileHandle),
				fileHandleWithStandardOutput
			]);
			NSFileHandle(p)
		}
	}

	#[must_use]
	pub fn from_null() -> NSFileHandle {
		unsafe {
			let p = StrongPtr::retain(msg_send![class!(NSFileHandle), fileHandleWithNullDevice]);
			NSFileHandle(p)
		}
	}
}

pub struct NSDictionary(pub StrongPtr);
unsafe impl Send for NSDictionary {}

impl NSDictionary {
	#[must_use]
	pub fn all_keys<T>(&self) -> NSArray<T> {
		unsafe {
			NSArray {
				p: StrongPtr::retain(msg_send![*self.0, allKeys]),
				_phantom: PhantomData,
			}
		}
	}

	#[must_use]
	pub fn all_values<T>(&self) -> NSArray<T> {
		unsafe {
			NSArray {
				p: StrongPtr::retain(msg_send![*self.0, allValues]),
				_phantom: PhantomData,
			}
		}
	}
}

#[derive(Deref, From)]
#[deref(forward)]
pub struct NSError(StrongPtr);
unsafe impl Send for NSError {}

impl NSError {
	#[must_use]
	pub fn nil() -> NSError {
		unsafe {
			let p = StrongPtr::new(NIL);
			NSError(p)
		}
	}

	#[allow(clippy::not_unsafe_ptr_arg_deref)] // Safe, because we null-check `id`
	pub fn result_from_nullable(id: Id) -> Result<(), NSError> {
		if id.is_null() {
			Ok(())
		} else {
			let strong = unsafe { StrongPtr::retain(id) };
			Err(NSError::from(strong))
		}
	}

	#[must_use]
	pub fn code(&self) -> isize {
		unsafe { msg_send![*self.0, code] }
	}

	#[must_use]
	pub fn localized_description(&self) -> NSString {
		unsafe { NSString(StrongPtr::retain(msg_send![*self.0, localizedDescription])) }
	}

	#[must_use]
	pub fn localized_failure_reason(&self) -> NSString {
		unsafe {
			NSString(StrongPtr::retain(msg_send![
				*self.0,
				localizedFailureReason
			]))
		}
	}

	#[must_use]
	pub fn localized_recovery_suggestion(&self) -> NSString {
		unsafe {
			NSString(StrongPtr::retain(msg_send![
				*self.0,
				localizedRecoverySuggestion
			]))
		}
	}

	#[must_use]
	pub fn help_anchor(&self) -> NSString {
		unsafe { NSString(StrongPtr::retain(msg_send![*self.0, helpAnchor])) }
	}

	#[must_use]
	pub fn user_info(&self) -> NSDictionary {
		unsafe { NSDictionary(StrongPtr::retain(msg_send![*self.0, userInfo])) }
	}

	pub fn dump(&self) {
		let code = self.code();
		println!("code: {}", code);
		let localized_description = self.localized_description();
		println!("localizedDescription : {}", localized_description.as_str());
		let localized_failure_reason = self.localized_failure_reason();
		println!(
			"localizedFailureReason : {}",
			localized_failure_reason.as_str()
		);
		let localized_recovery_suggestion = self.localized_recovery_suggestion();
		println!(
			"localizedRecoverySuggestion : {}",
			localized_recovery_suggestion.as_str()
		);
		let help_anchor = self.help_anchor();
		println!("helpAnchor : {}", help_anchor.as_str());
		let user_info = self.user_info();
		println!("userInfo :");
		let keys: NSArray<NSString> = user_info.all_keys();
		let values: NSArray<NSString> = user_info.all_values();
		let cnt = keys.count();
		for i in 0..cnt {
			let k = keys.object_at_index(i);
			let o = values.object_at_index(i);
			println!("    key: {}, value: {}", k.as_str(), o.as_str());
		}
	}
}

impl From<NSError> for anyhow::Error {
	fn from(err: NSError) -> anyhow::Error {
		use anyhow::anyhow;
		anyhow!(
			"{} (code: {})",
			err.localized_description().as_str(),
			err.code()
		)
	}
}

// Bind to libdispatch
#[link(name = "Foundation", kind = "framework")]
extern "C" {
	pub static QOS_CLASS_USER_INTERACTIVE: usize;
	pub static QOS_CLASS_USER_INITIATED: usize;
	pub static QOS_CLASS_UTILITY: usize;
	pub static QOS_CLASS_BACKGROUND: usize;

	pub fn dispatch_queue_create(label: *const c_char, attr: *const Object) -> Id;
	pub fn dispatch_get_global_queue(identifier: usize, flags: usize) -> Id;
	pub fn dispatch_retain(queue: Id);
	pub fn dispatch_release(queue: Id);
	pub fn dispatch_async_f(queue: Id, context: *mut c_void, function: extern "C" fn(*mut c_void));
	pub fn dispatch_sync_f(queue: Id, context: *mut c_void, function: extern "C" fn(*mut c_void));

}

#[derive(Deref, From)]
pub struct DispatchQueue(Id);
unsafe impl Send for DispatchQueue {}

/// > Dispatch queues themselves are thread safe. In other words, you can submit
/// > tasks to a dispatch queue from any thread on the system without first taking
/// > a lock or synchronizing access to the queue
/// From: <https://developer.apple.com/library/archive/documentation/General/Conceptual/ConcurrencyProgrammingGuide/OperationQueues/OperationQueues.html>
unsafe impl Sync for DispatchQueue {}

impl DispatchQueue {
	#[must_use]
	pub fn new_serial(label: &'static str) -> DispatchQueue {
		let label = label.as_ptr().cast::<i8>();
		let queue_id = unsafe { dispatch_queue_create(label, std::ptr::null()) };
		DispatchQueue(queue_id)
	}

	/// Get the global user-interactive dispatch queue
	#[must_use]
	pub fn user_interactive() -> DispatchQueue {
		let queue_id = unsafe { dispatch_get_global_queue(QOS_CLASS_USER_INTERACTIVE, 0) };
		DispatchQueue(queue_id)
	}

	/// Get the global user-initiated dispatch queue
	#[must_use]
	pub fn user_initiated() -> DispatchQueue {
		let queue_id = unsafe { dispatch_get_global_queue(QOS_CLASS_USER_INITIATED, 0) };
		DispatchQueue(queue_id)
	}

	/// Get the global utility dispatch queue
	#[must_use]
	pub fn utility() -> DispatchQueue {
		let queue_id = unsafe { dispatch_get_global_queue(QOS_CLASS_UTILITY, 0) };
		DispatchQueue(queue_id)
	}

	/// Get the global background dispatch queue
	#[must_use]
	pub fn background() -> DispatchQueue {
		let queue_id = unsafe { dispatch_get_global_queue(QOS_CLASS_BACKGROUND, 0) };
		DispatchQueue(queue_id)
	}

	pub fn dispatch_async<F>(&self, f: F)
	where
		F: FnOnce() + Send + 'static,
	{
		unsafe {
			let (context, function) = context_and_function(f);
			dispatch_async_f(**self, context, function);
		}
	}

	pub fn dispatch_sync<F>(&self, f: F)
	where
		F: FnOnce() + Send,
	{
		unsafe {
			let (context, function) = context_and_function(f);

			// Inform Tokio that we're going to block on the result of this computation
			tokio::task::block_in_place(|| dispatch_sync_f(**self, context, function));
		}
	}

	pub async fn promise<F, R>(&self, f: F) -> Result<R, oneshot::error::RecvError>
	where
		F: FnOnce(PromiseHandle<R>) + Send,
		R: Send,
	{
		let (handle, recv_result) = PromiseHandle::new();
		self.dispatch_sync(|| f(handle));
		recv_result.await
	}
}

#[derive(Clone)]
pub struct PromiseHandle<T> {
	// SAFETY: It is unsafe to panic while holding this mutex.
	sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<T>>>>,
}

impl<T> PromiseHandle<T> {
	fn new() -> (PromiseHandle<T>, oneshot::Receiver<T>) {
		let (send_result, recv_result) = oneshot::channel::<T>();
		let handle = PromiseHandle {
			sender: Arc::new(Mutex::new(Some(send_result))),
		};
		(handle, recv_result)
	}

	pub fn resolve(&self, value: T) {
		// SAFETY: We never panic while holding this mutex.
		let mut handle = unsafe { self.sender.lock().unwrap_unchecked() };
		let optional_sender = handle.take();
		drop(handle);

		if let Some(sender) = optional_sender {
			// Drop the value if it could not be sent due to the receiver having been dropped.
			// This is fine---double-resolving is a no-op.
			drop(sender.send(value));
		}

		// If we're not first, do nothing. All sends other than the first are no-ops.
	}
}

impl Drop for DispatchQueue {
	fn drop(&mut self) {
		// Decrement the refcount on the queue.
		// If this queue is a global queue, calling this function has no effect---which is the
		// desired behavior.
		unsafe { dispatch_release(**self) };
	}
}

impl Clone for DispatchQueue {
	fn clone(&self) -> Self {
		// Increment the reference count before copying.
		unsafe { dispatch_retain(**self) };
		Self(**self)
	}
}

/// Convert a closure into a heap-allocated context, and a function pointer callable with that
/// context as its argument.
///
/// This lets you call a C API defined like this:
///
/// ```c
/// dispatch_something(void* context, void (*function)(void*))
/// ```
///
/// Adapted from: <https://github.com/SSheldon/rust-dispatch/blob/f540a2d8ccaebf0e87f5805033b9e287e8d01ba5/src/lib.rs#L91-L102>
///
/// # Safety
///
///  - The caller is responsible for enforcing the appropriate `Send` and/or `'static` bounds on
///  the closure
///  - The caller must make sure that the C function pointer is eventually called in order to drop
///  the closure
///
unsafe fn context_and_function<F>(closure: F) -> (*mut c_void, extern "C" fn(*mut c_void))
where
	F: FnOnce(), // NOTE: the caller must enforce the appropriate `Send` or `'static` bounds
{
	// Define a C function that just calls a boxed closure.
	extern "C" fn work_execute_closure<F>(context: Box<F>)
	where
		F: FnOnce(),
	{
		use std::panic::{catch_unwind, AssertUnwindSafe};

		// Run the boxed closure
		// `context` is always `drop`-ed here, freeing the closure.
		let result = catch_unwind(AssertUnwindSafe(|| (*context)()));

		// If the closure panics, immediately abort.
		// It's unsafe to unwind into Objective-C.
		if result.is_err() {
			eprintln!("Aborting: Rust panic caught unwinding into Objective-C");
			drop(std::io::Write::flush(&mut std::io::stderr().lock()));
			std::process::abort();
		}
	}
	let func: extern "C" fn(Box<F>) = work_execute_closure::<F>;

	let closure = Box::new(closure);

	// Transmute the context into a void*
	let context = std::mem::transmute(closure);

	// Transmute the function into function pointer that takes a single void*
	let function: extern "C" fn(*mut c_void) -> () = std::mem::transmute(func);

	(context, function)
}

/// For debugging purposes, build a string describing all information known about a class by the
/// runtime.
///
/// Docs on type encodings:
/// <https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/ObjCRuntimeGuide/Articles/ocrtTypeEncodings.html>
#[cfg(debug_assertions)]
#[must_use]
pub fn _describe_all_fields(class: &Class) -> String {
	use std::fmt::Write;
	let mut s = String::new();
	writeln!(s, "=== Describe '{}' ===", class.name()).unwrap();
	writeln!(
		s,
		"superclass:    {:?}",
		class.superclass().map(Class::name)
	)
	.unwrap();
	writeln!(s, "metaclass:     {:?}", class.metaclass().name()).unwrap();
	writeln!(s, "instance_size: {:}", class.instance_size()).unwrap();

	writeln!(s).unwrap();
	writeln!(s, "instance_variables:").unwrap();
	for x in class.instance_variables().iter() {
		writeln!(s, "  name: {:20}  type: {:?}", x.name(), x.type_encoding()).unwrap();
	}

	writeln!(s).unwrap();
	writeln!(s, "instance_methods:").unwrap();
	for x in class.instance_methods().iter() {
		let name = x.name().name().to_string();
		let return_type = x.return_type();
		write!(s, "  name: {name:30}  ret: {return_type:?}  args: ",).unwrap();
		for n in 0..x.arguments_count() {
			write!(s, "{:?},", x.argument_type(n).unwrap()).unwrap();
		}
		writeln!(s).unwrap();
	}

	writeln!(s, "========================").unwrap();

	s
}
