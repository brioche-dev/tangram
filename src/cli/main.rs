#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]

use self::{commands::Args, error::Result};
use clap::Parser;
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;
mod error;

struct Cli {
	client: tg::Client,
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
		let mut error: &dyn std::error::Error = &error;
		loop {
			eprintln!("{error}");
			if let Some(source) = error.source() {
				error = source;
			} else {
				break;
			}
		}

		// Exit with a non-zero code.
		std::process::exit(1);
	}
}

async fn main_inner() -> Result<()> {
	// Parse the arguments.
	let args = Args::parse();

	let client = tg::Client::new_remote("http://localhost:8476".parse().unwrap(), None);

	// Create the CLI.
	let cli = Cli { client };

	// Run the command.
	cli.run(args).await?;

	Ok(())
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
