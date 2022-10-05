#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::{collections::BTreeMap, path::PathBuf};
use tangram::{builder::Builder, hash::Hash, system::System};
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod credentials;
mod dirs;

#[derive(Parser)]
#[command(
	about = env!("CARGO_PKG_DESCRIPTION"),
	disable_help_subcommand = true,
	long_version = env!("CARGO_PKG_VERSION"),
	name = env!("CARGO_CRATE_NAME"),
	version = env!("CARGO_PKG_VERSION"),
)]
struct Args {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
	Autoshell(commands::autoshell::Args),
	Blob(commands::blob::Args),
	Build(commands::build::Args),
	Checkin(commands::checkin::Args),
	Checkout(commands::checkout::Args),
	Expression(commands::expression::Args),
	Fetch(commands::fetch::Args),
	Gc(commands::gc::Args),
	Hash(commands::hash::Args),
	Init(commands::init::Args),
	// Login(commands::login::Args),
	New(commands::new::Args),
	// Publish(commands::publish::Args),
	Run(commands::run::Args),
	Search(commands::search::Args),
	Serve(commands::serve::Args),
	Shell(commands::shell::Args),
	Update(commands::update::Args),
	Upgrade(commands::upgrade::Args),
	Push(commands::push::Args),
	Pull(commands::pull::Args),
}

#[tokio::main]
async fn main() -> Result<()> {
	// Enable backtraces in debug mode.
	if cfg!(debug_assertions) && std::env::var_os("RUST_BACKTRACE").is_none() {
		std::env::set_var("RUST_BACKTRACE", "1");
	}

	// Setup tracing.
	setup_tracing();

	let args = Args::parse();
	match args.subcommand {
		Subcommand::Autoshell(args) => commands::autoshell::run(args).boxed(),
		Subcommand::Build(args) => commands::build::run(args).boxed(),
		Subcommand::Blob(args) => commands::blob::run(args).boxed(),
		Subcommand::Checkin(args) => commands::checkin::run(args).boxed(),
		Subcommand::Checkout(args) => commands::checkout::run(args).boxed(),
		Subcommand::Expression(args) => commands::expression::run(args).boxed(),
		Subcommand::Fetch(args) => commands::fetch::run(args).boxed(),
		Subcommand::Gc(args) => commands::gc::run(args).boxed(),
		Subcommand::Hash(args) => commands::hash::run(args).boxed(),
		Subcommand::Init(args) => commands::init::run(args).boxed(),
		// Subcommand::Login(args) => commands::login::run(args).boxed(),
		Subcommand::New(args) => commands::new::run(args).boxed(),
		// Subcommand::Publish(args) => commands::publish::run(args).boxed(),
		Subcommand::Pull(args) => commands::pull::run(args).boxed(),
		Subcommand::Push(args) => commands::push::run(args).boxed(),
		Subcommand::Run(args) => commands::run::run(args).boxed(),
		Subcommand::Search(args) => commands::search::run(args).boxed(),
		Subcommand::Serve(args) => commands::serve::run(args).boxed(),
		Subcommand::Shell(args) => commands::shell::run(args).boxed(),
		Subcommand::Update(args) => commands::update::run(args).boxed(),
		Subcommand::Upgrade(args) => commands::upgrade::run(args).boxed(),
	}
	.await?;
	Ok(())
}

pub fn path() -> Result<PathBuf> {
	Ok(crate::dirs::home_directory_path()
		.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
		.join(".tangram"))
}

pub fn config_path() -> Result<PathBuf> {
	Ok(path()?.join("config.json"))
}

pub fn credentials_path() -> Result<PathBuf> {
	Ok(path()?.join("credentials.json"))
}

pub async fn builder() -> Result<Builder> {
	// Get the path.
	let path = path()?;

	// Read the config.
	let config_path = config_path()?;
	let config = Config::read(&config_path)
		.await
		.context("Failed to read the config.")?;

	// Create the builder.
	let builder = Builder::new(tangram::config::Config {
		path,
		peers: config.peers,
	})
	.await
	.context("Failed to create the builder.")?;

	Ok(builder)
}

pub async fn create_target_args(
	builder: &tangram::builder::Shared,
	system: Option<System>,
) -> Result<Hash> {
	let mut target_arg = BTreeMap::new();
	let system = if let Some(system) = system {
		system
	} else {
		System::host()?
	};
	let system = builder
		.add_expression(&tangram::expression::Expression::String(
			system.to_string().into(),
		))
		.await?;
	target_arg.insert("system".into(), system);
	let target_arg = builder
		.add_expression(&tangram::expression::Expression::Map(target_arg))
		.await?;
	let target_args = vec![target_arg];
	let target_args = builder
		.add_expression(&tangram::expression::Expression::Array(target_args))
		.await?;
	Ok(target_args)
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
