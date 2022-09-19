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
	version = concat!(env!("CARGO_PKG_VERSION")),
	disable_help_subcommand = true,
)]
struct Args {
	#[clap(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
	Build(commands::build::Args),
	Checkin(commands::checkin::Args),
	Checkout(commands::checkout::Args),
	Fetch(commands::fetch::Args),
	Gc(commands::gc::Args),
	New(commands::new::Args),
	Publish(commands::publish::Args),
	Run(commands::run::Args),
	Search(commands::search::Args),
	Server(commands::server::Args),
	Shell(commands::shell::Args),
}

#[tokio::main]
async fn main() -> Result<()> {
	// Enable backtraces by default in debug mode.
	if cfg!(debug_assertions)
		&& matches!(
			std::env::var("RUST_BACKTRACE"),
			Err(std::env::VarError::NotPresent)
		) {
		std::env::set_var("RUST_BACKTRACE", "1");
	}
	setup_tracing();

	let args = Args::parse();
	match args.subcommand {
		Subcommand::Build(args) => commands::build::run(args).boxed(),
		Subcommand::Checkin(args) => commands::checkin::run(args).boxed(),
		Subcommand::Checkout(args) => commands::checkout::run(args).boxed(),
		Subcommand::Fetch(args) => commands::fetch::run(args).boxed(),
		Subcommand::Gc(args) => commands::gc::run(args).boxed(),
		Subcommand::New(args) => commands::new::run(args).boxed(),
		Subcommand::Publish(args) => commands::publish::run(args).boxed(),
		Subcommand::Run(args) => commands::run::run(args).boxed(),
		Subcommand::Search(args) => commands::search::run(args).boxed(),
		Subcommand::Server(args) => commands::server::run(args).boxed(),
		Subcommand::Shell(args) => commands::shell::run(args).boxed(),
	}
	.await?;
	Ok(())
}

fn setup_tracing() {
	let env_layer = if std::env::var("TANGRAM_TRACING").is_ok() {
		let filter =
			tracing_subscriber::filter::EnvFilter::try_from_env("TANGRAM_TRACING").unwrap();
		Some(filter)
	} else if cfg!(debug_assertions) {
		Some(tracing_subscriber::EnvFilter::new("[]=info"))
	} else {
		None
	};
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
