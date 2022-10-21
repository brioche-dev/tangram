#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::path::PathBuf;
use tangram_core::{
	api_client::ApiClient,
	builder::{self, clients, Builder},
};

mod commands;
mod config;
mod credentials;
mod dirs;
mod util;

pub struct Cli {
	builder: Builder,
	api_client: ApiClient,
}

#[derive(Parser)]
#[command(
	about = env!("CARGO_PKG_DESCRIPTION"),
	disable_help_subcommand = true,
	long_version = env!("CARGO_PKG_VERSION"),
	name = env!("CARGO_CRATE_NAME"),
	version = env!("CARGO_PKG_VERSION"),
)]
pub struct Args {
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
	Autoshell(commands::autoshell::Args),
	Blob(commands::blob::Args),
	Build(commands::build::Args),
	Check(commands::check::Args),
	Checkin(commands::checkin::Args),
	Checkout(commands::checkout::Args),
	Expression(commands::expression::Args),
	Fetch(commands::fetch::Args),
	Gc(commands::gc::Args),
	Hash(commands::hash::Args),
	Init(commands::init::Args),
	Login(commands::login::Args),
	Lsp(commands::lsp::Args),
	New(commands::new::Args),
	Publish(commands::publish::Args),
	Pull(commands::pull::Args),
	Push(commands::push::Args),
	Repl(commands::repl::Args),
	Run(commands::run::Args),
	Search(commands::search::Args),
	Shell(commands::shell::Args),
	Update(commands::update::Args),
	Upgrade(commands::upgrade::Args),
}

impl Cli {
	pub async fn new() -> Result<Cli> {
		// Get the CLI path.
		let path = Self::path()?;

		// Read the config file.
		let config = Self::read_config().await?;

		// Resolve the autoshells.
		let autoshells = config
			.as_ref()
			.and_then(|config| config.autoshells.as_ref())
			.cloned();
		let _autoshells = autoshells.unwrap_or_default();

		// Resolve the API URL.
		let api_url = config
			.as_ref()
			.and_then(|config| config.api_url.as_ref())
			.cloned();
		let api_url = api_url.unwrap_or_else(|| "https://api.tangram.dev".parse().unwrap());

		// Read the credentials.
		let credentials = Self::read_credentials().await?;

		// Get the token.
		let token = credentials.map(|credentials| credentials.token);

		// Create the API Client.
		let api_client = ApiClient::new(api_url.clone(), token.clone());

		// Create the blob client.
		let blob_client = clients::blob::Client::new(api_url.clone(), token.clone());

		// Create the expression client.
		let expression_client = clients::expression::Client::new(api_url.clone(), token.clone());

		// Create the builder.
		let options = builder::Options {
			blob_client: Some(blob_client),
			expression_client: Some(expression_client),
		};
		let builder = Builder::new(path, options)
			.await
			.context("Failed to create the builder.")?;

		// Create the CLI.
		let cli = Cli {
			builder,
			api_client,
		};

		Ok(cli)
	}

	fn path() -> Result<PathBuf> {
		Ok(crate::dirs::home_directory_path()
			.context("Failed to find the user home directory.")?
			.join(".tangram"))
	}
}

impl Cli {
	/// Run a command.
	pub async fn run_command(&self, args: Args) -> Result<()> {
		// Run the subcommand.
		match args.subcommand {
			Subcommand::Autoshell(args) => self.command_autoshell(args).boxed(),
			Subcommand::Blob(args) => self.command_blob(args).boxed(),
			Subcommand::Build(args) => self.command_build(args).boxed(),
			Subcommand::Check(args) => self.command_check(args).boxed(),
			Subcommand::Checkin(args) => self.command_checkin(args).boxed(),
			Subcommand::Checkout(args) => self.command_checkout(args).boxed(),
			Subcommand::Expression(args) => self.command_expression(args).boxed(),
			Subcommand::Fetch(args) => self.command_fetch(args).boxed(),
			Subcommand::Gc(args) => self.command_gc(args).boxed(),
			Subcommand::Hash(args) => self.command_hash(args).boxed(),
			Subcommand::Init(args) => self.command_init(args).boxed(),
			Subcommand::Login(args) => self.command_login(args).boxed(),
			Subcommand::Lsp(args) => self.command_lsp(args).boxed(),
			Subcommand::New(args) => self.command_new(args).boxed(),
			Subcommand::Publish(args) => self.command_publish(args).boxed(),
			Subcommand::Pull(args) => self.command_pull(args).boxed(),
			Subcommand::Push(args) => self.command_push(args).boxed(),
			Subcommand::Repl(args) => self.command_repl(args).boxed(),
			Subcommand::Run(args) => self.command_run(args).boxed(),
			Subcommand::Search(args) => self.command_search(args).boxed(),
			Subcommand::Shell(args) => self.command_shell(args).boxed(),
			Subcommand::Update(args) => self.command_update(args).boxed(),
			Subcommand::Upgrade(args) => self.command_upgrade(args).boxed(),
		}
		.await?;
		Ok(())
	}
}
