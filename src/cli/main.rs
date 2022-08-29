#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use anyhow::Result;
use clap::Parser;
use futures::FutureExt;
use tracing_subscriber::prelude::*;

mod client;
mod commands;
mod dirs;

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
	Repl(commands::repl::Args),
	Server(commands::server::Args),
}

#[tokio::main]
async fn main() -> Result<()> {
	// Enable backtraces by default in debug mode.
	if cfg!(debug_assertions) && std::env::var_os("RUST_BACKTRACE").is_none() {
		std::env::set_var("RUST_BACKTRACE", "1");
	}
	setup_tracing();

	let args = Args::parse();
	match args.subcommand {
		Subcommand::Build(args) => commands::build::run(args).boxed(),
		Subcommand::Checkin(args) => commands::checkin::run(args).boxed(),
		Subcommand::Checkout(args) => commands::checkout::run(args).boxed(),
		Subcommand::Fetch(args) => commands::fetch::run(args).boxed(),
		Subcommand::Repl(args) => commands::repl::run(args).boxed(),
		Subcommand::Server(args) => commands::server::run(args).boxed(),
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
