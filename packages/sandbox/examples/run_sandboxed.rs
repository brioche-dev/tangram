use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use clap::Parser;
use std::os::unix::process::ExitStatusExt;
use tangram_hash::Hash;
use tangram_sandbox::Id;

#[derive(Parser)]
struct Args {
	/// Don't enforce sandbox rules, only log violations to kernel logs.
	#[clap(long)]
	advisory: bool,

	#[clap(long, default_value("_fakeprefix/artifacts"))]
	artifact_root: Utf8PathBuf,

	#[clap(
		long("artifact"),
		default_value("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
	)]
	artifacts: Vec<Hash>,

	#[clap(long, default_value("_fakeprefix/fragments"))]
	fragment_root: Utf8PathBuf,

	#[clap(long, default_value("00000000000000000000000000000000"))]
	workdir_fragment: Id,

	#[clap(long, default_value("11111111111111111111111111111111"))]
	output_fragment: Id,

	#[clap(long)]
	network: bool,

	#[clap(long, parse(try_from_str = parse_key_val), number_of_values=1)]
	env: Vec<(String, String)>,

	program: String,

	args: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();

	// Make sure artifacts, fragments roots exist.
	tokio::fs::create_dir_all(&args.artifact_root)
		.await
		.context("failed to create artifact root dir")?;
	tokio::fs::create_dir_all(&args.fragment_root)
		.await
		.context("failed to create fragment root dir")?;
	tokio::fs::create_dir_all(&args.fragment_root.join(&args.workdir_fragment.to_string()))
		.await
		.context("failed to create workdir fragment")?;
	for hash in &args.artifacts {
		tokio::fs::create_dir_all(&args.artifact_root.join(hash.to_string()))
			.await
			.context("failed to create artifact dir")?;
	}

	let cmd = tangram_sandbox::Command {
		artifact_root: args.artifact_root,
		artifacts: args.artifacts.clone(),

		fragment_root: args.fragment_root,
		workdir_fragment: args.workdir_fragment,
		output_fragment: args.output_fragment,

		env: args.env.into_iter().collect(),
		network: args.network,
		args: args.args,
		program: args.program.clone(),

		// Pipe through the child stdio
		pipe_stdio: false,
	};

	eprintln!("sandbox: tangram_sandbox::{cmd:#?}");
	for artifact in &args.artifacts {
		eprintln!("sandbox: add '{artifact}' to $PATH with:");
		eprintln!(r#"         export PATH=$PWD/../../artifacts/{artifact}/bin:$PATH"#);
	}
	eprintln!("sandbox: start '{}'", &args.program);
	eprintln!();

	let sandbox = match args.advisory {
		true => tangram_sandbox::macos::MacosSandbox::new_advisory(),
		false => tangram_sandbox::macos::MacosSandbox::new(),
	};
	let mut child = sandbox.spawn(&cmd).await?;

	let result = child
		.wait()
		.await
		.context("failed to wait for child output")?;

	match result.code() {
		Some(code) => std::process::exit(code),
		None => {
			eprintln!(
				"run_sandboxed: child exited with signal: {}",
				result.signal().unwrap()
			);
			std::process::exit(1);
		},
	}
}

/// Parse a single key-value pair
/// From: <https://github.com/clap-rs/clap_derive/blob/master/examples/keyvalue.rs>
fn parse_key_val<T, U>(s: &str) -> Result<(T, U)>
where
	T: std::str::FromStr,
	T::Err: std::error::Error + 'static + Send + Sync,
	U: std::str::FromStr,
	U::Err: std::error::Error + 'static + Send + Sync,
{
	let pos = s.find('=').context("failed to split")?;
	Ok((
		s[..pos].parse().context("failed to parse key")?,
		s[pos + 1..].parse().context("failed to parse value")?,
	))
}
