use crate::Cli;
use anyhow::Result;
use clap::Parser;
use either::Either;
use futures::FutureExt;
use std::path::PathBuf;

pub mod add;
pub mod autoshell;
pub mod blob;
pub mod build;
pub mod check;
pub mod checkin;
pub mod checkout;
pub mod download;
pub mod dump_metadata;
pub mod fmt;
pub mod gc;
pub mod init;
pub mod lint;
pub mod login;
pub mod lsp;
pub mod new;
pub mod outdated;
pub mod publish;
pub mod pull;
pub mod push;
pub mod run;
pub mod search;
pub mod serve;
pub mod shell;
pub mod update;
pub mod upgrade;

#[derive(Parser)]
#[command(
	about = env!("CARGO_PKG_DESCRIPTION"),
	disable_help_subcommand = true,
	long_version = env!("CARGO_PKG_VERSION"),
	name = env!("CARGO_CRATE_NAME"),
	version = env!("CARGO_PKG_VERSION"),
)]
pub struct Args {
	#[arg(
		long,
		help = "The path where tangram should store its data. Defaults to $HOME/.tangram."
	)]
	pub path: Option<PathBuf>,
	#[command(subcommand)]
	pub command: Command,
}

#[derive(Parser)]
pub enum Command {
	Add(self::add::Args),
	Autoshell(self::autoshell::Args),
	Blob(self::blob::Args),
	Build(self::build::Args),
	Check(self::check::Args),
	Checkin(self::checkin::Args),
	Checkout(self::checkout::Args),
	Download(self::download::Args),
	DumpMetadata(self::dump_metadata::Args),
	Fmt(self::fmt::Args),
	Gc(self::gc::Args),
	Init(self::init::Args),
	Lint(self::lint::Args),
	Login(self::login::Args),
	Lsp(self::lsp::Args),
	New(self::new::Args),
	Outdated(self::outdated::Args),
	Publish(self::publish::Args),
	Pull(self::pull::Args),
	Push(self::push::Args),
	Run(self::run::Args),
	Search(self::search::Args),
	Serve(self::serve::Args),
	Shell(self::shell::Args),
	Update(self::update::Args),
	Upgrade(self::upgrade::Args),
}

impl Cli {
	/// Run a command.
	pub async fn run_command(&self, args: Args) -> Result<()> {
		// Acquire an appropriate lock for the subcommand.
		let _lock = match args.command {
			Command::Gc(_) => {
				if let Some(lock) = self.try_lock_exclusive().await? {
					Either::Left(lock)
				} else {
					eprintln!("Waiting on an exclusive lock to the tangram path.");
					Either::Left(self.lock_exclusive().await?)
				}
			},
			_ => {
				if let Some(lock) = self.try_lock_shared().await? {
					Either::Right(lock)
				} else {
					eprintln!("Waiting on a shared lock to the tangram path.");
					Either::Right(self.lock_shared().await?)
				}
			},
		};

		// Run the subcommand.
		match args.command {
			Command::Add(args) => self.command_add(args).boxed(),
			Command::Autoshell(args) => self.command_autoshell(args).boxed(),
			Command::Blob(args) => self.command_blob(args).boxed(),
			Command::Build(args) => self.command_build(args).boxed(),
			Command::Check(args) => self.command_check(args).boxed(),
			Command::Checkin(args) => self.command_checkin(args).boxed(),
			Command::Checkout(args) => self.command_checkout(args).boxed(),
			Command::Download(args) => self.command_download(args).boxed(),
			Command::DumpMetadata(args) => self.command_dump_metadata(args).boxed(),
			Command::Fmt(args) => self.command_fmt(args).boxed(),
			Command::Gc(args) => self.command_gc(args).boxed(),
			Command::Init(args) => self.command_init(args).boxed(),
			Command::Lint(args) => self.command_lint(args).boxed(),
			Command::Login(args) => self.command_login(args).boxed(),
			Command::Lsp(args) => self.command_lsp(args).boxed(),
			Command::New(args) => self.command_new(args).boxed(),
			Command::Outdated(args) => self.command_outdated(args).boxed(),
			Command::Publish(args) => self.command_publish(args).boxed(),
			Command::Pull(args) => self.command_pull(args).boxed(),
			Command::Push(args) => self.command_push(args).boxed(),
			Command::Run(args) => self.command_run(args).boxed(),
			Command::Search(args) => self.command_search(args).boxed(),
			Command::Serve(args) => self.command_serve(args).boxed(),
			Command::Shell(args) => self.command_shell(args).boxed(),
			Command::Update(args) => self.command_update(args).boxed(),
			Command::Upgrade(args) => self.command_upgrade(args).boxed(),
		}
		.await?;
		Ok(())
	}
}
