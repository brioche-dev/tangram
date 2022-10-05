#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use anyhow::Result;
use clap::Parser;
use tangram_cli::{Args, Cli};
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
	// Enable backtraces in debug mode.
	if cfg!(debug_assertions) && std::env::var_os("RUST_BACKTRACE").is_none() {
		std::env::set_var("RUST_BACKTRACE", "1");
	}

	// Setup tracing.
	setup_tracing();

	// Parse the arguments.
	let args = Args::parse();

	// Create the CLI.
	let cli = Cli::new().await?;

	// Run the CLI.
	cli.run(args).await?;

	Ok(())
}

fn setup_tracing() {
	// Create the env layer.
	let env_layer = if std::env::var("TANGRAM_TRACING").is_ok() {
		let filter =
			tracing_subscriber::filter::EnvFilter::try_from_env("TANGRAM_TRACING").unwrap();
		Some(filter)
	} else if cfg!(debug_assertions) {
		Some(tracing_subscriber::EnvFilter::new("[]=info"))
	} else {
		None
	};

	// If tracing is enabled, create and initialize the subscriber.
	if let Some(env_layer) = env_layer {
		let format_layer = tracing_subscriber::fmt::layer()
			.pretty()
			.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NEW);
		let subscriber = tracing_subscriber::registry()
			.with(env_layer)
			.with(format_layer);
		subscriber.init();
	}
}
