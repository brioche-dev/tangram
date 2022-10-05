#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::{collections::BTreeMap, path::PathBuf};
use tangram_api_client::ApiClient;
use tangram_core::{builder::Builder, hash::Hash, system::System};

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
pub struct Args {
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
	Login(commands::login::Args),
	New(commands::new::Args),
	Publish(commands::publish::Args),
	Run(commands::run::Args),
	Search(commands::search::Args),
	Serve(commands::serve::Args),
	Shell(commands::shell::Args),
	Update(commands::update::Args),
	Upgrade(commands::upgrade::Args),
	Push(commands::push::Args),
	Pull(commands::pull::Args),
}

pub struct Cli {
	config: Config,
	builder: Builder,
	api_client: ApiClient,
}

impl Cli {
	#[must_use]
	pub async fn new() -> Result<Cli> {
		// Read the config.
		todo!()
	}

	pub async fn run(&self, args: Args) -> Result<()> {
		match args.subcommand {
			Subcommand::Autoshell(args) => self.command_autoshell(args).boxed(),
			Subcommand::Build(args) => self.command_build(args).boxed(),
			Subcommand::Blob(args) => self.command_blob(args).boxed(),
			Subcommand::Checkin(args) => self.command_checkin(args).boxed(),
			Subcommand::Checkout(args) => self.command_checkout(args).boxed(),
			Subcommand::Expression(args) => self.command_expression(args).boxed(),
			Subcommand::Fetch(args) => self.command_fetch(args).boxed(),
			Subcommand::Gc(args) => self.command_gc(args).boxed(),
			Subcommand::Hash(args) => self.command_hash(args).boxed(),
			Subcommand::Init(args) => self.command_init(args).boxed(),
			Subcommand::Login(args) => self.command_login(args).boxed(),
			Subcommand::New(args) => self.command_new(args).boxed(),
			Subcommand::Publish(args) => self.command_publish(args).boxed(),
			Subcommand::Pull(args) => self.command_pull(args).boxed(),
			Subcommand::Push(args) => self.command_push(args).boxed(),
			Subcommand::Run(args) => self.command_run(args).boxed(),
			Subcommand::Search(args) => self.command_search(args).boxed(),
			Subcommand::Serve(args) => self.command_serve(args).boxed(),
			Subcommand::Shell(args) => self.command_shell(args).boxed(),
			Subcommand::Update(args) => self.command_update(args).boxed(),
			Subcommand::Upgrade(args) => self.command_upgrade(args).boxed(),
		}
		.await?;
		Ok(())
	}
}

fn path() -> Result<PathBuf> {
	Ok(crate::dirs::home_directory_path()
		.ok_or_else(|| anyhow!("Failed to find the user home directory."))?
		.join(".tangram"))
}

fn config_path() -> Result<PathBuf> {
	Ok(path()?.join("config.json"))
}

fn credentials_path() -> Result<PathBuf> {
	Ok(path()?.join("credentials.json"))
}

async fn builder() -> Result<Builder> {
	// Get the path.
	let path = path()?;

	// Read the config.
	let config_path = config_path()?;
	let config = Config::read(&config_path)
		.await
		.context("Failed to read the config.")?;

	// Create the builder.
	let builder = Builder::new(tangram_core::options::Options {
		path,
		peers: config.peers,
	})
	.await
	.context("Failed to create the builder.")?;

	Ok(builder)
}

// async fn client(url: Option<Url>) -> Result<Client> {
// 	let (url, token) = if let Some(url) = url {
// 		(url, None)
// 	} else {
// 		// Read the config.
// 		let config_path = config_path()?;
// 		let config = Config::read(&config_path)
// 			.await
// 			.context("Failed to read the config.")?;
// 			let credentials = credentials();
// 		(config.api_url,
// 	};

// 	Client::new(url)
// }

async fn api_client() -> Result<ApiClient> {
	// Get the path.
	let path = path()?;

	// Read the config.
	let config_path = config_path()?;
	let config = Config::read(&config_path)
		.await
		.context("Failed to read the config.")?;

	let api_client = ApiClient::new(config.api_url, "".to_owned());

	Ok(api_client)
}

async fn create_target_args(
	builder: &tangram_core::builder::Shared,
	system: Option<System>,
) -> Result<Hash> {
	let mut target_arg = BTreeMap::new();
	let system = if let Some(system) = system {
		system
	} else {
		System::host()?
	};
	let system = builder
		.add_expression(&tangram_core::expression::Expression::String(
			system.to_string().into(),
		))
		.await?;
	target_arg.insert("system".into(), system);
	let target_arg = builder
		.add_expression(&tangram_core::expression::Expression::Map(target_arg))
		.await?;
	let target_args = vec![target_arg];
	let target_args = builder
		.add_expression(&tangram_core::expression::Expression::Array(target_args))
		.await?;
	Ok(target_args)
}
