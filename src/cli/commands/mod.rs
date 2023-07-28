use crate::{error::Result, Cli};
use futures::FutureExt;
use std::path::PathBuf;
use tangram::path::Subpath;

mod add;
mod autoenv;
mod blob;
mod block;
mod build;
mod check;
mod checkin;
mod checkout;
mod checksum;
mod clean;
mod doc;
mod download;
mod env;
mod exec;
mod fmt;
mod init;
mod log;
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
mod test;
mod tree;
mod update;
mod upgrade;

#[derive(Debug, clap::Parser)]
#[command(
	about = env!("CARGO_PKG_DESCRIPTION"),
	disable_help_subcommand = true,
	long_version = env!("CARGO_PKG_VERSION"),
	name = env!("CARGO_CRATE_NAME"),
	verbatim_doc_comment,
	version = env!("CARGO_PKG_VERSION"),
)]
pub struct Args {
	/// The path where Tangram should store its data. The default is `$HOME/.tangram`.
	#[arg(long, env = "TANGRAM_PATH")]
	pub path: Option<PathBuf>,

	/// If set, temp files and directories will persist after the process exits.
	#[arg(long, env = "TANGRAM_PRESERVE_TEMPS")]
	pub preserve_temps: Option<bool>,

	/// If set, child processes will be run un-sandboxed.
	#[arg(long, env = "TANGRAM_SANDBOX_ENABLED")]
	pub sandbox_enabled: Option<bool>,

	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
	Add(self::add::Args),
	Autoenv(self::autoenv::Args),
	Blob(self::blob::Args),
	Block(self::block::Args),
	Build(self::build::Args),
	Check(self::check::Args),
	Checkin(self::checkin::Args),
	Checkout(self::checkout::Args),
	Checksum(self::checksum::Args),
	Clean(self::clean::Args),
	Doc(self::doc::Args),
	Download(self::download::Args),
	Env(self::env::Args),
	Exec(self::exec::Args),
	Fmt(self::fmt::Args),
	Init(self::init::Args),
	Log(self::log::Args),
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
	Test(self::test::Args),
	Tree(self::tree::Args),
	Update(self::update::Args),
	Upgrade(self::upgrade::Args),
}

#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct PackageArgs {
	// /// If this flag is set, the package's lockfile will not be updated before building.
	// #[arg(long)]
	// pub locked: bool,
}

#[derive(Debug, clap::Args)]
#[command(verbatim_doc_comment)]
pub struct RunArgs {
	/// The path to the executable in the artifact to run.
	#[arg(long)]
	pub executable_path: Option<Subpath>,
}

impl Cli {
	/// Run a command.
	#[tracing::instrument(skip_all)]
	pub async fn run(&self, args: Args) -> Result<()> {
		// Run the subcommand.
		match args.command {
			Command::Add(args) => self.command_add(args).boxed(),
			Command::Autoenv(args) => self.command_autoenv(args).boxed(),
			Command::Blob(args) => self.command_blob(args).boxed(),
			Command::Block(args) => self.command_block(args).boxed(),
			Command::Build(args) => self.command_build(args).boxed(),
			Command::Check(args) => self.command_check(args).boxed(),
			Command::Checkin(args) => self.command_checkin(args).boxed(),
			Command::Checkout(args) => self.command_checkout(args).boxed(),
			Command::Checksum(args) => self.command_checksum(args).boxed(),
			Command::Clean(args) => self.command_clean(args).boxed(),
			Command::Doc(args) => self.command_doc(args).boxed(),
			Command::Download(args) => self.command_download(args).boxed(),
			Command::Env(args) => self.command_env(args).boxed(),
			Command::Exec(args) => self.command_exec(args).boxed(),
			Command::Fmt(args) => self.command_fmt(args).boxed(),
			Command::Init(args) => self.command_init(args).boxed(),
			Command::Log(args) => self.command_log(args).boxed(),
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
			Command::Test(args) => self.command_test(args).boxed(),
			Command::Tree(args) => self.command_tree(args).boxed(),
			Command::Update(args) => self.command_update(args).boxed(),
			Command::Upgrade(args) => self.command_upgrade(args).boxed(),
		}
		.await?;
		Ok(())
	}
}
