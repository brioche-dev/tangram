#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_pattern)]

use self::{
	commands::{Args, Command},
	util::dirs::home_directory_path,
};
use clap::Parser;
use tangram_client as tg;
use tangram_util::addr::Addr;
use tg::{error, Result, WrapErr};
use tracing_subscriber::prelude::*;

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

	// Initialize V8.
	initialize_v8();

	// Parse the arguments.
	let args = Args::parse();

	// Create the tokio runtime.
	let rt = tokio::runtime::Runtime::new().expect("Failed to create the tokio runtime.");

	// Run the CLI.
	let result = rt.block_on(main_inner(args));

	// Handle the result.
	if let Err(error) = result {
		// Print the error trace.
		eprintln!("An error occurred.");
		eprintln!("{}", error.trace());

		// Exit with a non-zero code.
		std::process::exit(1);
	}
}

async fn main_inner(args: Args) -> Result<()> {
	// Get the path.
	let path = home_directory_path()
		.wrap_err("Failed to find the user home directory.")?
		.join(".tangram");
	let addr = Addr::Unix(path.join("socket"));

	// Create the client.
	let client = if let Command::Serve(_) = &args.command {
		None
	} else {
		Some(
			Box::new(tangram_client::remote::Remote::new(addr.clone(), None).await?)
				as Box<dyn tg::Client>,
		)
	};

	// Create the CLI.
	let cli = Cli { client };

	// Run the command.
	cli.run(args).await
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
