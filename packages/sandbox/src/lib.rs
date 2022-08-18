//! Cross-platform sandboxing for Tangram builds.
//!
//! # What's available
//!
//! Inside the sandbox, builds have access to a limited environment. This environment includes:
//!
//! - Read-only access to a list of Tangram artifacts
//! - Read-write access to a temporary Tangram fragment serving as that build's working directory
//! - Read-write access to a Tangram fragment for build outputs, whose path is passed to the child
//! through the `$OUT` environment variable
//! - Access to the network (if enabled)
//! - Read-only access to essential system files (like `/dev/null`)
//! - Read-only access to essential shell tools (in detail below)
//!
//! # Essential Shell Tools
//!
//! Currently, the list of system utilities exposed as-is to the sandbox is:
//!
//! - (macOS) `/bin/zsh`: the Z shell
//!     - We use `zsh` in POSIX `sh`-compatible mode to run scripts on macOS.
//!
//! These utilities are not bundled with Tangram in any way---they are expected to be found on the
//! host system `$PATH`, and to work as specified in POSIX.
//!
//! # Examples
//!
//! To run a command in a macOS sandbox:
//!
//! ```rust
//! # #[tokio::main]
//! # async fn main() {
//! // Configure the sandboxed command
//! let cmd = tangram_sandbox::Command {
//!     // Echo 'Hello, World!' from /bin/zsh, using an empty environment.
//!     program: "/bin/zsh".into(),
//!     args: vec!["-c".into(), "echo 'Hello, World!'".into()],
//!     env: std::collections::BTreeMap::new(),
//!
//!     // Allow access to an artifact
//!     artifact_root: "_fakeprefix/artifacts".into(),
//!     artifacts: vec![
//!             "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
//!                 .parse().unwrap(),
//!     ],
//!
//!     // Specify the fragment to use for the working directory and output
//!     fragment_root: "_fakeprefix/fragments".into(),
//!     workdir_fragment: "00000000000000000000000000000000".parse().unwrap(),
//!     output_fragment: "11111111111111111111111111111111".parse().unwrap(),
//!
//!     // Allow access to the network
//!     network: true,
//!
//!     // Capture the child process stdio
//!     pipe_stdio: true,
//! };
//!
//! // Run the command in a macOS sandbox
//! let mut child = tangram_sandbox::macos::MacosSandbox::new()
//!     .spawn(&cmd)
//!     .await
//!     .unwrap();
//!
//! // Read the stdout of the command
//! let mut stdout = Vec::new();
//! tokio::io::copy(&mut child.stdout.take().unwrap(), &mut stdout).await.unwrap();
//!
//! // Wait for completion, check exit code
//! let result = child.wait().await.unwrap();
//! assert!(result.success(), "child failed to execute");
//!
//! // Check the output is right
//! assert_eq!(std::str::from_utf8(&stdout).unwrap(), "Hello, World!\n");
//! # }
//! ```

use anyhow::{anyhow, ensure, Context, Result};
use camino::Utf8PathBuf;
use std::collections::BTreeMap;
use tangram_hash::Hash;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

/// Sandbox configuration used to run a command.
#[derive(Debug, Clone)]
pub struct Command {
	/// Directory storing artifacts, as `$artifact_root/$hash`
	pub artifact_root: Utf8PathBuf,

	/// List of artifacts which sandboxed programs can read from.
	pub artifacts: Vec<Hash /* TODO: ArtifactHash, pending main refactor */>,

	/// Directory storing fragments, as `$fragment_root/$id`
	pub fragment_root: Utf8PathBuf,

	/// The ID of the fragment to use as this build's working directory.
	///
	/// The child process will start with this fragment's path as its
	/// [`current_dir`](tokio::process::Command::current_dir)
	pub workdir_fragment: Id,

	/// The ID of the fragment to use as this build's output directory.
	///
	/// The path to this fragment inside the sandbox will be passed to the child process as the
	/// `$OUT` environment variable.
	pub output_fragment: Id,

	/// Whether or not sandboxed programs can access the network.
	pub network: bool,

	/// Whether we should pipe the stdio of the child back to the parent, rather than allowing the
	/// child to inherit the parent's stdio streams.
	pub pipe_stdio: bool,

	/// The name of the program to run.
	///
	/// For resolution rules, see [`Command::new`](std::process::Command::new).
	pub program: String,

	/// Arguments to the program.
	pub args: Vec<String>,

	/// The program's complete set of environment variables.
	///
	/// Sandboxed commands inherit no environment variables from the parent process.
	///
	/// Before starting the child, we will insert one special environment variable, `$OUT`, which
	/// contains the path inside the sandbox to the output fragment.
	pub env: BTreeMap<String, String>,
}

/// Find the absolute paths of all the [essential shell tools][crate#essential-shell-tools]
async fn essential_tool_paths() -> Result<Vec<Utf8PathBuf>> {
	// We block_in_place because the `which` logic is blocking
	let mut paths = Vec::new();

	// Always use /bin/zsh
	let sh_path = Utf8PathBuf::from("/bin/zsh");
	ensure!(sh_path.exists(), "/bin/zsh does not exist");
	paths.push(sh_path);

	Ok(paths)
}

/// Fragment ID
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Id(u128); // TODO: move `tangram_core::Id` to another crate, pending main refactor

impl std::fmt::Debug for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self)
	}
}

impl std::fmt::Display for Id {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:032x?}", self.0)
	}
}

impl std::str::FromStr for Id {
	type Err = anyhow::Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.len() != 32 {
			return Err(anyhow!("wrong length"));
		}
		let id = u128::from_str_radix(s, 16).context("failed to parse")?;
		let id = Id(id);
		Ok(id)
	}
}
