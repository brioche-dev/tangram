use crate::Cli;
use anyhow::Result;
use clap::Parser;
use futures::FutureExt;

pub mod add;
pub mod autoshell;
pub mod blob;
pub mod build;
pub mod check;
pub mod checkin;
pub mod checkout;
pub mod download;
pub mod fmt;
pub mod gc;
pub mod init;
pub mod lint;
// pub mod login;
pub mod lsp;
pub mod new;
pub mod outdated;
// pub mod publish;
// pub mod pull;
// pub mod push;
pub mod run;
// pub mod search;
// pub mod serve;
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
	#[command(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
enum Subcommand {
	Add(self::add::Args),
	Autoshell(self::autoshell::Args),
	Blob(self::blob::Args),
	Build(self::build::Args),
	Check(self::check::Args),
	Checkin(self::checkin::Args),
	Checkout(self::checkout::Args),
	Download(self::download::Args),
	Fmt(self::fmt::Args),
	Gc(self::gc::Args),
	Init(self::init::Args),
	Lint(self::lint::Args),
	// Login(self::login::Args),
	Lsp(self::lsp::Args),
	New(self::new::Args),
	Outdated(self::outdated::Args),
	// Publish(self::publish::Args),
	// Pull(self::pull::Args),
	// Push(self::push::Args),
	Run(self::run::Args),
	// Search(self::search::Args),
	// Serve(self::serve::Args),
	Shell(self::shell::Args),
	Update(self::update::Args),
	Upgrade(self::upgrade::Args),
}

impl Cli {
	/// Run a command.
	pub async fn run_command(&self, args: Args) -> Result<()> {
		// Run the subcommand.
		match args.subcommand {
			Subcommand::Add(args) => self.command_add(args).boxed(),
			Subcommand::Autoshell(args) => self.command_autoshell(args).boxed(),
			Subcommand::Blob(args) => self.command_blob(args).boxed(),
			Subcommand::Build(args) => self.command_build(args).boxed(),
			Subcommand::Check(args) => self.command_check(args).boxed(),
			Subcommand::Checkin(args) => self.command_checkin(args).boxed(),
			Subcommand::Checkout(args) => self.command_checkout(args).boxed(),
			Subcommand::Download(args) => self.command_download(args).boxed(),
			Subcommand::Fmt(args) => self.command_fmt(args).boxed(),
			Subcommand::Gc(args) => self.command_gc(args).boxed(),
			Subcommand::Init(args) => self.command_init(args).boxed(),
			Subcommand::Lint(args) => self.command_lint(args).boxed(),
			// Subcommand::Login(args) => self.command_login(args).boxed(),
			Subcommand::Lsp(args) => self.command_lsp(args).boxed(),
			Subcommand::New(args) => self.command_new(args).boxed(),
			Subcommand::Outdated(args) => self.command_outdated(args).boxed(),
			// Subcommand::Publish(args) => self.command_publish(args).boxed(),
			// Subcommand::Pull(args) => self.command_pull(args).boxed(),
			// Subcommand::Push(args) => self.command_push(args).boxed(),
			Subcommand::Run(args) => self.command_run(args).boxed(),
			// Subcommand::Search(args) => self.command_search(args).boxed(),
			// Subcommand::Serve(args) => self.command_serve(args).boxed(),
			Subcommand::Shell(args) => self.command_shell(args).boxed(),
			Subcommand::Update(args) => self.command_update(args).boxed(),
			Subcommand::Upgrade(args) => self.command_upgrade(args).boxed(),
		}
		.await?;
		Ok(())
	}
}
