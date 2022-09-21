#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use anyhow::Result;
use clap::Parser;
use futures::FutureExt;
use tracing_subscriber::prelude::*;

mod commands;
mod config;
mod dirs;
mod util;

#[derive(Parser)]
#[clap(
	about = env!("CARGO_PKG_DESCRIPTION"),
	disable_help_subcommand = true,
	long_version = env!("CARGO_PKG_VERSION"),
	name = env!("CARGO_CRATE_NAME"),
	version = env!("CARGO_PKG_VERSION"),
)]
struct Args {
	#[clap(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
	Autoshell(commands::autoshell::Args),
	Build(commands::build::Args),
	Checkin(commands::checkin::Args),
	Checkout(commands::checkout::Args),
	Fetch(commands::fetch::Args),
	Gc(commands::gc::Args),
	Init(commands::init::Args),
	Publish(commands::publish::Args),
	Run(commands::run::Args),
	Search(commands::search::Args),
	Server(commands::server::Args),
	Shell(commands::shell::Args),
	Shellhook(commands::shellhook::Args),
	Update(commands::update::Args),
	Upgrade(commands::upgrade::Args),
}

#[tokio::main]
async fn main() -> Result<()> {
	// Enable backtraces in debug mode.
	if cfg!(debug_assertions) {
		std::env::set_var("RUST_BACKTRACE", "1");
	}

	// Setup tracing.
	setup_tracing();

	let args = Args::parse();
	match args.subcommand {
		Subcommand::Autoshell(args) => commands::autoshell::run(args).boxed(),
		Subcommand::Build(args) => commands::build::run(args).boxed(),
		Subcommand::Checkin(args) => commands::checkin::run(args).boxed(),
		Subcommand::Checkout(args) => commands::checkout::run(args).boxed(),
		Subcommand::Fetch(args) => commands::fetch::run(args).boxed(),
		Subcommand::Gc(args) => commands::gc::run(args).boxed(),
		Subcommand::Init(args) => commands::init::run(args).boxed(),
		Subcommand::Publish(args) => commands::publish::run(args).boxed(),
		Subcommand::Run(args) => commands::run::run(args).boxed(),
		Subcommand::Search(args) => commands::search::run(args).boxed(),
		Subcommand::Server(args) => commands::server::run(args).boxed(),
		Subcommand::Shell(args) => commands::shell::run(args).boxed(),
		Subcommand::Shellhook(args) => commands::shellhook::run(args).boxed(),
		Subcommand::Update(args) => commands::update::run(args).boxed(),
		Subcommand::Upgrade(args) => commands::upgrade::run(args).boxed(),
	}
	.await?;
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
