use super::{c_void, crash, crash_errno, socket_send, top_stack_addr, waitpid, SandboxContext};

pub extern "C" fn child_callback(arg: *mut c_void) -> libc::c_int {
	let ctx = unsafe { &mut *arg.cast::<crate::process::linux::SandboxContext<'_>>() };
	unsafe { child_main(ctx) }
}

unsafe fn child_main(ctx: &mut SandboxContext) -> i32 {
	// Ask to be SIGKILL'd if the parent process exits.
	let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL, 0, 0, 0);
	if ret == -1 {
		crash_errno!("Failed to set PDEATHSIG")
	}

	// Set the stdio file descriptors with `dup2`.
	let ret = libc::dup2(ctx.stdio_fd, libc::STDOUT_FILENO);
	if ret == -1 {
		crash_errno!("Failed to set stdout to the stdio_fd");
	}
	let ret = libc::dup2(ctx.stdio_fd, libc::STDERR_FILENO);
	if ret == -1 {
		crash_errno!("Failed to set stderr to the stdio_fd");
	}

	// Close stdin.
	let ret = libc::close(libc::STDIN_FILENO);
	if ret == -1 {
		crash_errno!("Failed to close stdin.");
	}

	// If we're not passing through network access to the guest, pass CLONE_NEWNET to isolate the guest's network namespace.
	let network_clone_flags = if ctx.network_enabled {
		0
	} else {
		libc::CLONE_NEWNET
	};

	// Spawn the inner child.
	let inner_child_pid: libc::pid_t = libc::clone(
		super::inner::child_callback,
		top_stack_addr(ctx.inner_child_stack),
		libc::CLONE_NEWNS | libc::CLONE_NEWPID | network_clone_flags,
		ctx as *mut _ as *mut c_void,
	);
	if inner_child_pid == -1 {
		crash_errno!("Failed to spawn inner child");
	}

	// Send the inner child's PID, so the parent can write the UID and GID maps.
	let result = socket_send(ctx.coordination_socket_inner_fd, &inner_child_pid);
	if let Err(e) = result {
		crash!("Failed to send PID of inner child: {}", e);
	}

	// Wait for the inner child to complete.
	let inner_child_exit = match waitpid(inner_child_pid) {
		Err(e) => crash!("Failed to wait for inner child from outer child: {}", e),
		Ok(exit) => exit,
	};

	// Send the parent the exit code of the inner child.
	let result = socket_send(ctx.coordination_socket_inner_fd, &inner_child_exit);
	if let Err(e) = result {
		crash!(
			"Failed to send inner child exit status back to parent: {}",
			e
		);
	}

	0
}
