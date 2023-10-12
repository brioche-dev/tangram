#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_pattern)]

use self::{commands::Args, util::dirs::home_directory_path};
use clap::Parser;
use tangram_client as tg;
use tg::error::{error, return_error, Result, WrapErr};
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;
mod util;

pub const API_URL: &str = "https://api.tangram.dev";

struct Cli {
	client: Box<dyn tg::Client>,
}

#[tokio::main]
async fn main() {
	// Setup tracing.
	setup_tracing();

	// Run the main function.
	let result = main_inner().await;

	// If an error occurred, print the error trace and exit with a non-zero code.
	if let Err(error) = result {
		// Print the error trace.
		eprintln!("An error occurred.");
		eprintln!("{}", error.trace());

		// Exit with a non-zero code.
		std::process::exit(1);
	}
}

async fn main_inner() -> Result<()> {
	// Parse the arguments.
	let args = Args::parse();

	// Initialize V8.
	initialize_v8();

	// Create the server.
	let path = home_directory_path().unwrap().join(".tangram");
	let server = tangram_server::Server::new(path, None).await?;

	// Create the CLI.
	let cli = Cli {
		client: Box::new(server),
	};

	// Run the command.
	cli.run(args).await?;

	Ok(())
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
