use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use tangram_core::config::{
	Config, DEFAULT_DAEMON_GROUP_NAME, DEFAULT_DAEMON_ROOT_PATH, DEFAULT_DAEMON_USER_NAME,
};
use tangram_installer::{init_daemon, init_nodaemon, uninit_daemon, uninit_nodaemon};
use tangram_io::fs;

#[derive(Parser)]
pub struct Args {
	#[clap(subcommand)]
	pub cmd: Subcommand,
}

#[tokio::main]
pub async fn main() -> Result<()> {
	tracing_subscriber::FmtSubscriber::builder()
		.with_target(false)
		.with_max_level(tracing::Level::WARN)
		.without_time()
		.init();

	let args = Args::parse();
	match args.cmd {
		Subcommand::Init(args) => init(args).await,
		Subcommand::Uninit(args) => uninit(args).await,
	}
}

#[derive(Parser)]
pub enum Subcommand {
	Init(InitArgs),
	Uninit(UninitArgs),
}

#[derive(Parser)]
pub struct InitArgs {
	/// Disable daemon mode (the default)
	#[clap(long)]
	no_daemon: bool,

	/// The name of the user to create for the Tangram daemon.
	#[clap(long)]
	user_name: Option<String>,

	/// The name of the group to create for the Tangram daemon.
	#[clap(long)]
	group_name: Option<String>,

	/// The location of the tangram store.
	#[clap(long)]
	root_path: Option<PathBuf>,
}

pub async fn init(args: InitArgs) -> Result<()> {
	let user_name: &str = args
		.user_name
		.as_deref()
		.unwrap_or(DEFAULT_DAEMON_USER_NAME);
	let group_name: &str = args
		.group_name
		.as_deref()
		.unwrap_or(DEFAULT_DAEMON_GROUP_NAME);

	let config = Config {
		daemon: Some(!args.no_daemon),
		root_path: args.root_path,
	};
	config
		.write()
		.await
		.context("failed to write config file")?;

	if config.daemon == Some(false) {
		let root_path = config
			.root_path
			.unwrap_or_else(|| tangram_dirs::data_dir().unwrap().join("tangram"));
		init_nodaemon(root_path)?;
	} else {
		let root_path = config
			.root_path
			.unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_ROOT_PATH));
		init_daemon(user_name, group_name, &root_path)?;
	}
	Ok(())
}

#[derive(Parser)]
pub struct UninitArgs {
	/// The name of the user to remove
	user_name: Option<String>,

	/// The name of the group to remove.
	group_name: Option<String>,
}

pub async fn uninit(args: UninitArgs) -> Result<()> {
	let user_name: &str = args
		.user_name
		.as_deref()
		.unwrap_or(DEFAULT_DAEMON_USER_NAME);
	let group_name: &str = args
		.group_name
		.as_deref()
		.unwrap_or(DEFAULT_DAEMON_GROUP_NAME);

	let config = Config::read().await.context("failed to read config file")?;

	// Remove the config file
	fs::remove_file(Config::config_file_path()?)
		.await
		.context("failed to delete config file")?;

	if config.daemon == Some(false) {
		let root_path = config
			.root_path
			.unwrap_or_else(|| tangram_dirs::data_dir().unwrap().join("tangram"));
		uninit_nodaemon(root_path)?;
	} else {
		let root_path = config
			.root_path
			.unwrap_or_else(|| PathBuf::from(DEFAULT_DAEMON_ROOT_PATH));
		uninit_daemon(user_name, group_name, &root_path)?;
	}
	Ok(())
}
