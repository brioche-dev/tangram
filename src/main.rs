#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use anyhow::{Context, Result};
use clap::Parser;
use std::sync::Arc;
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
	let args = tangram::Args::parse();

	// Get the path.
	let path = if let Some(path) = args.path.clone() {
		path
	} else {
		tangram::os::dirs::home_directory_path()
			.context("Failed to find the user home directory.")?
			.join(".tangram")
	};

	// Create the CLI.
	let cli = Arc::new(tangram::Cli::new(path).await?);

	// Run the command.
	cli.command(args).await?;

	Ok(())
}

fn setup_tracing() {
	// Create the env layer.
	let env_layer = if std::env::var("TANGRAM_TRACING").is_ok() {
		let filter =
			tracing_subscriber::filter::EnvFilter::try_from_env("TANGRAM_TRACING").unwrap();
		Some(filter)
	} else if cfg!(debug_assertions) {
		Some(tracing_subscriber::EnvFilter::new("[]=off,tangram=info"))
	} else {
		None
	};

	// If tracing is enabled, then create and initialize the subscriber.
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
