//! MacOS enforcing and advisory sandboxing.

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use tokio::process;

mod policy;
use policy::PolicyBuilder;

/// A MacOS sandbox.
///
/// Use [`MacosSandbox::spawn`] to spawn a [`Command`](crate::Command).
///
pub struct MacosSandbox {
	/// Whether or not the sandbox enforces policy, or just logs violations to the system console.
	enforcing: bool,
}

impl MacosSandbox {
	/// Create a new [`MacosSandbox`]
	pub fn new() -> MacosSandbox {
		MacosSandbox { enforcing: true }
	}

	/// Create a new [`MacosSandbox`], set only to log sandbox violations, not block them.
	pub fn new_advisory() -> MacosSandbox {
		MacosSandbox { enforcing: false }
	}

	/// Spawn a command in the sandbox.
	pub async fn spawn(&self, spec: &crate::Command) -> Result<process::Child> {
		// Create the right type of policy, based on whether the sandbox is enforcing or advisory.
		let mut policy = match self.enforcing {
			true => PolicyBuilder::new(),
			false => PolicyBuilder::new_advisory(),
		};

		// Allow read access to the essential shell tools
		for tool in crate::essential_tool_paths()
			.await
			.context("failed to find paths to essential tools")?
		{
			policy
				.allow_read(&tool)
				.with_context(|| format!("failed to allow read access to tool: {tool}"))?;
		}

		// Allow read access to the artifact and fragment directories themselves.
		let fragment_root: Utf8PathBuf = tokio::fs::canonicalize(&spec.fragment_root)
			.await
			.context("failed to get canonical path for fragment root")?
			.try_into()
			.context("artifact root path was invalid utf-8")?;
		policy
			.allow_read(&fragment_root)
			.context("failed to allow read access to fragment root")?;
		let artifact_root: Utf8PathBuf = tokio::fs::canonicalize(&spec.artifact_root)
			.await
			.context("failed to get canonical path for artifact root")?
			.try_into()
			.context("artifact root path was invalid utf-8")?;
		policy
			.allow_read(&artifact_root)
			.context("failed to allow read access to artifact root")?;

		// Allow read-only access to artifact dependencies.
		for hash in &spec.artifacts {
			let path = spec.artifact_root.join(hash.to_string());
			let path: Utf8PathBuf = tokio::fs::canonicalize(path)
				.await
				.with_context(|| format!("failed to get canonical path for artifact: hash {hash}"))?
				.try_into()
				.with_context(|| {
					format!("canonical path for artifact was invalid utf-8: hash {hash}")
				})?;
			policy
				.allow_read_subpath(&path)
				.with_context(|| format!("for artifact: {hash}"))
				.context("failed to configure artifact sandbox rule")?;
		}

		// Allow read-write access to the working directory fragment.
		let cwd_path = fragment_root.join(spec.workdir_fragment.to_string());
		policy
			.allow_write_subpath(&cwd_path)
			.context("failed to configure working directory sandbox rule")?;

		// Allow read-write access to the output fragment.
		// NOTE: This path does not yet exist on disk.
		let output_path = fragment_root.join(spec.output_fragment.to_string());
		policy
			.allow_write_subpath(&output_path)
			.context("failed to configure output directory sandbox rule")?;

		// Conditionally allow network access.
		if spec.network {
			policy.allow_network();
		}

		// Create the command.
		let mut cmd = process::Command::new(&spec.program);
		cmd.args(&spec.args);

		// Clear any inherited env vars, and set up the environment.
		cmd.env_clear();
		for (k, v) in &spec.env {
			cmd.env(k, v);
		}

		// TODO: configure PATH correctly...?
		//       we might need to expose an environment variable for the workdir.
		//       How are builds going to shell out to specifically-versioned binaries from other
		//       artifacts?

		// Insert the OUT env var, pointing to the path of the output fragment.
		cmd.env("OUT", &output_path);

		// Set the process's working directory to the path of the workdir fragment.
		cmd.current_dir(cwd_path);

		// If required, pipe stdio to the parent process, rather than ineriting the streams
		// of the parent.
		if spec.pipe_stdio {
			cmd.stderr(std::process::Stdio::piped());
			cmd.stdout(std::process::Stdio::piped());
			cmd.stdin(std::process::Stdio::piped());
		}

		// Kill the process if the child handle gets dropped.
		cmd.kill_on_drop(true);

		// TODO: set `cmd.uid(...)` to an appropriate user

		// Put the child in a sandbox by calling `sandbox_init` after `fork()` and before `exec()`
		unsafe {
			let policy_string = policy.to_string();
			cmd.pre_exec(move || -> std::io::Result<()> {
				sandbox_init::sandbox_init(&policy_string)
					.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
			});
		}

		cmd.spawn().context("failed to spawn subprocess")
	}
}

impl Default for MacosSandbox {
	fn default() -> Self {
		Self::new()
	}
}
