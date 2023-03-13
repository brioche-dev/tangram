use crate::Cli;
use either::Either;
use futures::FutureExt;
use tangram::{error::Result, util::fs};

mod add;
mod autoshell;
mod build;
mod check;
mod checkin;
mod checkout;
mod clean;
mod doc;
mod download;
mod fmt;
mod init;
mod login;
mod lsp;
mod new;
mod outdated;
mod publish;
mod pull;
mod push;
mod remove;
mod run;
mod search;
mod serve;
mod shell;
mod update;
mod upgrade;

#[derive(clap::Parser)]
#[command(
	about = env!("CARGO_PKG_DESCRIPTION"),
	disable_help_subcommand = true,
	long_version = env!("CARGO_PKG_VERSION"),
	name = env!("CARGO_CRATE_NAME"),
	version = env!("CARGO_PKG_VERSION"),
)]
pub struct Args {
	/// The path where Tangram should store its data. The default is `$HOME/.tangram`.
	#[arg(long)]
	pub path: Option<fs::PathBuf>,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
	Add(self::add::Args),
	Autoshell(self::autoshell::Args),
	Build(self::build::Args),
	Check(self::check::Args),
	Checkin(self::checkin::Args),
	Checkout(self::checkout::Args),
	Clean(self::clean::Args),
	Doc(self::doc::Args),
	Download(self::download::Args),
	Fmt(self::fmt::Args),
	Init(self::init::Args),
	// Log(self::log::Args),
	Login(self::login::Args),
	Lsp(self::lsp::Args),
	New(self::new::Args),
	Outdated(self::outdated::Args),
	Publish(self::publish::Args),
	Pull(self::pull::Args),
	Push(self::push::Args),
	Remove(self::remove::Args),
	Run(self::run::Args),
	Search(self::search::Args),
	Serve(self::serve::Args),
	Shell(self::shell::Args),
	Update(self::update::Args),
	Upgrade(self::upgrade::Args),
}

impl Cli {
	/// Run a command.
	pub async fn run(&self, args: Args) -> Result<()> {
		// Acquire an appropriate lock for the subcommand.
		let _lock = match args.command {
			Command::Clean(_) => {
				if let Some(lock) = self.tg.try_lock_exclusive().await? {
					Either::Left(lock)
				} else {
					eprintln!("Waiting on an exclusive lock to the tangram path.");
					Either::Left(self.tg.lock_exclusive().await?)
				}
			},
			_ => {
				if let Some(lock) = self.tg.try_lock_shared().await? {
					Either::Right(lock)
				} else {
					eprintln!("Waiting on a shared lock to the tangram path.");
					Either::Right(self.tg.lock_shared().await?)
				}
			},
		};

		// Run the subcommand.
		match args.command {
			Command::Add(args) => self.command_add(args).boxed(),
			Command::Autoshell(args) => self.command_autoshell(args).boxed(),
			Command::Build(args) => self.command_build(args).boxed(),
			Command::Check(args) => self.command_check(args).boxed(),
			Command::Checkin(args) => self.command_checkin(args).boxed(),
			Command::Checkout(args) => self.command_checkout(args).boxed(),
			Command::Clean(args) => self.command_clean(args).boxed(),
			Command::Doc(args) => self.command_doc(args).boxed(),
			Command::Download(args) => self.command_download(args).boxed(),
			Command::Fmt(args) => self.command_fmt(args).boxed(),
			Command::Init(args) => self.command_init(args).boxed(),
			// Command::Log(args) => self.command_log(args).boxed(),
			Command::Login(args) => self.command_login(args).boxed(),
			Command::Lsp(args) => self.command_lsp(args).boxed(),
			Command::New(args) => self.command_new(args).boxed(),
			Command::Outdated(args) => self.command_outdated(args).boxed(),
			Command::Publish(args) => self.command_publish(args).boxed(),
			Command::Pull(args) => self.command_pull(args).boxed(),
			Command::Push(args) => self.command_push(args).boxed(),
			Command::Remove(args) => self.command_remove(args).boxed(),
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
