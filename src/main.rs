#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use self::commands::Args;
use clap::Parser;
use std::{collections::BTreeMap, sync::Arc};
use tangram::{
	error::{Context, Result},
	system::System,
	value::Value,
	Instance,
};
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;

struct Cli {
	tg: Arc<Instance>,
}

#[tokio::main]
async fn main() -> Result<()> {
	// Enable backtraces when debug assertions are enabled.
	if cfg!(debug_assertions) && std::env::var_os("RUST_BACKTRACE").is_none() {
		std::env::set_var("RUST_BACKTRACE", "1");
	}

	// Setup tracing.
	setup_tracing();

	// Parse the arguments.
	let args = Args::parse();

	// Get the path.
	let path = if let Some(path) = args.path.clone() {
		path
	} else {
		tangram::os::dirs::home_directory_path()
			.context("Failed to find the user home directory.")?
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
