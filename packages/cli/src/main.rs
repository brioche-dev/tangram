#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_pattern)]

use self::commands::Args;
use clap::Parser;
use commands::Command;
use std::os::fd::AsRawFd;
use tangram_client as tg;
use tangram_util::addr::Addr;
use tg::{error, Result, WrapErr};
use tracing_subscriber::prelude::*;
use util::dirs::home_directory_path;

mod commands;
mod config;
mod credentials;
mod util;

pub const API_URL: &str = "https://api.tangram.dev";

struct Cli {
	client: Option<Box<dyn tg::Client>>,
}

fn main() {
	// Setup tracing.
	setup_tracing();

	// Run the program.
	let result = main_inner();

	if let Err(error) = result {
		// Print the error trace.
		eprintln!("An error occurred.");
		eprintln!("{}", error.trace());

		// Exit with a non-zero code.
		std::process::exit(1);
	}
}

fn main_inner() -> Result<()> {
	// Parse the arguments.
	let args = Args::parse();

	// Check if we need to daemonize this process. This must occur before tokio's runtime is initialized.
	if let Command::Serve(args) = &args.command {
		if args.daemonize {
			daemonize().wrap_err("Failed to daemonize the process.")?;
		}
	}

	// Create the tokio runtime.
	let rt = tokio::runtime::Runtime::new().expect("Failed to initialie tokio runtime.");

	// Run the CLI.
	rt.block_on(async move {
		// Initialize V8.
		initialize_v8();

		// Create the client.
		let client = if let Command::Serve(_) = &args.command {
			None
		} else {
			// Get the path.
			let path = home_directory_path()
				.wrap_err("Failed to find the user home directory.")?
				.join(".tangram");
			let addr = Addr::Socket(path.join("socket"));
			let client = tangram_client::remote::Remote::new(addr.clone(), None).await;

			// Create a server if we're in debug mode.
			if cfg!(debug_assertions) {
				if let Ok(client) = client {
					Some(Box::new(client) as Box<dyn tg::Client>)
				} else {
					let cli = Cli { client: None };
					let args = commands::serve::Args {
						command: commands::serve::Action::Start,
						addr: Some(addr.clone()),
						path: Some(path),
						daemonize: false,
					};
					tokio::task::spawn(async move { cli.command_serve(args).await });
					let client = tangram_client::remote::Remote::new(addr.clone(), None).await?;
					Some(Box::new(client) as Box<dyn tg::Client>)
				}
			} else {
				Some(Box::new(client?) as Box<dyn tg::Client>)
			}
		};

		// Create the CLI.
		let cli = Cli { client };

		// Run the command.
		cli.run(args).await
	})
}

fn initialize_v8() {
	// Set the ICU data.
	#[repr(C, align(16))]
	struct IcuData([u8; 10_631_872]);
	static ICU_DATA: IcuData = IcuData(*include_bytes!(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../lsp/src/icudtl.dat"
	)));
	v8::icu::set_common_data_73(&ICU_DATA.0).unwrap();

	// Initialize the platform.
	let platform = v8::new_default_platform(0, true);
	v8::V8::initialize_platform(platform.make_shared());

	// Initialize V8.
	v8::V8::initialize();
}

fn setup_tracing() {
	// Create the env layer.
	let tracing_env_filter = std::env::var("TANGRAM_TRACING").ok();
	let env_layer = tracing_env_filter
		.map(|env_filter| tracing_subscriber::filter::EnvFilter::try_new(env_filter).unwrap());

	// If tracing is enabled, create and initialize the subscriber.
	if let Some(env_layer) = env_layer {
		let format_layer = tracing_subscriber::fmt::layer()
			.compact()
			.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NEW)
			.with_writer(std::io::stderr);
		let subscriber = tracing_subscriber::registry()
			.with(env_layer)
			.with(format_layer);
		subscriber.init();
	}
}

fn daemonize() -> std::io::Result<()> {
	extern "C" {
		fn daemon(nochdir: i32, noclose: i32) -> i32;
	}
	// TODO: create a .pid file and lock it.
	let outfile = std::fs::File::options()
		.create(true)
		.append(false)
		.read(true)
		.write(true)
		.open("/tmp/tg.serve.out")?;

	unsafe {
		let outfile_fd = outfile.as_raw_fd();
		let stdout_fd = std::io::stdout().as_raw_fd();
		if libc::dup2(outfile_fd, stdout_fd) < 0 {
			return Err(std::io::Error::last_os_error());
		}
		let stderr_fd = std::io::stderr().as_raw_fd();
		if libc::dup2(outfile_fd, stderr_fd) < 0 {
			return Err(std::io::Error::last_os_error());
		}
		if daemon(0, 1) != 0 {
			return Err(std::io::Error::last_os_error());
		}
	}

	Ok(())
}
