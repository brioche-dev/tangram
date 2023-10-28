#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::redundant_pattern)]

use self::{commands::Args, util::dirs::home_directory_path};
use clap::Parser;
use std::sync::Arc;
use tangram_client as tg;
use tangram_util::net::Addr;
use tg::{error, Result, WrapErr};
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;
mod util;

pub const API_URL: &str = "https://api.tangram.dev";

struct Cli {
	client: tokio::sync::Mutex<Option<Arc<dyn tg::Client>>>,
}

#[tokio::main]
async fn main() {
	// Setup tracing.
	setup_tracing();

	// Initialize V8.
	initialize_v8();

	// Parse the arguments.
	let args = Args::parse();

	// Create the CLI.
	let cli = Cli {
		client: tokio::sync::Mutex::new(None),
	};

	// Run the command.
	let result = cli.run(args).await;

	// Handle the result.
	if let Err(error) = result {
		// Print the error trace.
		eprintln!("An error occurred.");
		eprintln!("{}", error.trace());

		// Exit with a non-zero code.
		std::process::exit(1);
	}
}

impl Cli {
	async fn client(&self) -> Result<Arc<dyn tg::Client>> {
		let mut client = self.client.lock().await;
		if let Some(client) = &*client {
			Ok(client.clone())
		} else {
			let path = home_directory_path()
				.wrap_err("Failed to find the user home directory.")?
				.join(".tangram");
			let addr = Addr::Unix(path.join("socket"));
			*client = Some(Arc::new(
				tangram_client::remote::Remote::new(addr, false, None).await?,
			));
			Ok(client.as_ref().unwrap().clone())
		}
	}
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
