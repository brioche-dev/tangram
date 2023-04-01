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
	// Setup tracing.
	setup_tracing();

	// Run the main function.
	let result = main_inner().await;

	// If an error occurred, print the error trace and exit with a non-zero status code.
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

		// Exit with a non-zero status code.
		std::process::exit(1);
	}
}

#[tracing::instrument(name = "main")]
async fn main_inner() -> Result<()> {
	// Parse the arguments.
	let args = Args::parse();

	tracing::debug!(?args, "Running command.");

	// Get the path.
	let path = if let Some(path) = args.path.clone() {
		path
	} else {
		tangram::util::dirs::home_directory_path()
			.wrap_err("Failed to find the user home directory.")?
			.join(".tangram")
	};

	tracing::debug!(?path, "Got path.");

	// Read the config.
	let config = Cli::read_config().await?;

	tracing::debug!(?config, "Read config.");

	// Read the credentials.
	let credentials = Cli::read_credentials().await?;

	// Resolve the API URL.
	let api_url = config
		.as_ref()
		.and_then(|config| config.api_url.as_ref())
		.cloned();

	// Get the token.
	let api_token = credentials.map(|credentials| credentials.token);

	tracing::debug!(?api_url, has_token = api_token.is_some(), "Got API config.");

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
	let tracing_env_filter = std::env::var("TANGRAM_TRACING").ok();
	let env_layer = tracing_env_filter
		.map(|env_filter| tracing_subscriber::filter::EnvFilter::try_new(env_filter).unwrap());

	// Creat the OpenTelemetry layer.
	let jaeger_endpoint = std::env::var("TANGRAM_OPENTELEMETRY_JAEGER").ok();
	let otel_layer = jaeger_endpoint.map(|jaeger_endpoint| {
		#[cfg(feature = "opentelemetry")]
		{
			let tracer = opentelemetry_jaeger::new_agent_pipeline()
				.with_service_name("tangram")
				.with_endpoint(jaeger_endpoint)
				.install_simple()
				.expect("Failed to set up OpenTelemtry pipeline.");
			tracing_opentelemetry::layer().with_tracer(tracer)
		}

		#[cfg(not(feature = "opentelemetry"))]
		{
			let _ = jaeger_endpoint;
			eprintln!("TANGRAM_OPENTELEMETRY_JAEGER is set, but the opentelemetry feature is not enabled.");
			tracing_subscriber::layer::Identity::new()
		}
	});

	// If tracing is enabled, create and initialize the subscriber.
	if let Some(env_layer) = env_layer {
		let format_layer = tracing_subscriber::fmt::layer()
			.compact()
			.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NEW)
			.with_writer(std::io::stderr);
		let subscriber = tracing_subscriber::registry()
			.with(env_layer)
			.with(format_layer)
			.with(otel_layer);
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
