#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use self::{commands::Args, error::Result};
use clap::Parser;
use std::{collections::BTreeMap, sync::Arc};
use tangram::{error::WrapErr, system::System, value::Value, Instance};
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;
mod error;

struct Cli {
	tg: Arc<Instance>,
}

#[tokio::main]
async fn main() {
	// Run the main function.
	let result = main_inner().await;

	// Handle the result.
	match result {
		Ok(_) => {},
		Err(error) => {
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

			// Exit with a non-zero status code.
			std::process::exit(1);
		},
	}
}

async fn main_inner() -> Result<()> {
	// Setup tracing.
	setup_tracing();

	// Parse the arguments.
	let args = Args::parse();

	// Get the path.
	let path = if let Some(path) = args.path.clone() {
		path
	} else {
		tangram::util::dirs::home_directory_path()
			.wrap_err("Failed to find the user home directory.")?
			.join(".tangram")
	};

	// Read the config.
	let config = Cli::read_config_from_path(&path.join("config.json")).await?;

	// Read the credentials.
	let credentials = Cli::read_credentials_from_path(&path.join("credentials.json")).await?;

	// Resolve the API URL.
	let api_url = config
		.as_ref()
		.and_then(|config| config.api_url.as_ref())
		.cloned();

	// Get the token.
	let api_token = credentials.map(|credentials| credentials.token);

	// Create the options.
	let options = tangram::Options { api_url, api_token };

	// Create the instance.
	let tg = Arc::new(tangram::Instance::new(path, options).await?);

	// Create the CLI.
	let cli = Cli { tg };

	// Run the command.
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

impl Cli {
	fn create_default_context() -> Result<BTreeMap<String, Value>> {
		let host = System::host()?;
		let host = Value::String(host.to_string());
		let context = [("host".to_owned(), host)].into();
		Ok(context)
	}
}
