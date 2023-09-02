#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_safety_doc)]

use self::{commands::Args, error::Result};
use clap::Parser;
use std::collections::BTreeMap;
use tg::{error::WrapErr, instance::Instance, system::System, value::Value};
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;
mod error;

struct Cli {
	tg: Instance,
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

	// Get the path.
	let path = if let Some(path) = args.path.clone() {
		path
	} else {
		tg::util::dirs::home_directory_path()
			.wrap_err("Failed to find the user home directory.")?
			.join(".tangram")
	};

	// Read the config.
	let config = Cli::read_config().await?;

	// Get the preserve temps configuration.
	let preserve_temps = args
		.preserve_temps
		.or(config.as_ref().and_then(|c| c.preserve_temps))
		.unwrap_or(false);

	// Get the sandbox configuration.
	let sandbox_enabled = args
		.sandbox_enabled
		.or(config.as_ref().and_then(|c| c.sandbox_enabled))
		.unwrap_or(true);

	// Read the credentials.
	let credentials = Cli::read_credentials().await?;

	// Get the origin URL.
	let origin_url = config
		.as_ref()
		.and_then(|config| config.origin_url.as_ref())
		.cloned();

	// Get the origin token.
	let origin_token = credentials.map(|credentials| credentials.token);

	// Create the options.
	let options = tg::instance::Options {
		origin_token,
		origin_url,
		preserve_temps,
		sandbox_enabled,
	};

	// Create the instance.
	let tg = Instance::new(path, options).await?;

	// Create the CLI.
	let cli = Cli { tg };

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

impl Cli {
	fn create_default_env() -> Result<BTreeMap<String, Value>> {
		Ok([("host".to_owned(), Value::from(System::host()?.to_string()))].into())
	}
}
