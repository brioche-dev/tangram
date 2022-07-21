use anyhow::Result;
use clap::Parser;
use futures::FutureExt;

#[derive(Parser)]
pub struct Args {
	#[clap(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	List(self::list::Args),
	Create(self::create::Args),
	Delete(self::delete::Args),
	Start(self::start::Args),
	Stop(self::stop::Args),
}

pub async fn run(args: Args) -> Result<()> {
	match args.subcommand {
		Subcommand::List(args) => self::list::list(args).boxed(),
		Subcommand::Create(args) => self::create::create(args).boxed(),
		Subcommand::Delete(args) => self::delete::delete(args).boxed(),
		Subcommand::Start(args) => self::start::start(args).boxed(),
		Subcommand::Stop(args) => self::stop::stop(args).boxed(),
	}
	.await?;
	Ok(())
}

pub mod list {
	#[allow(clippy::wildcard_imports)]
	use super::*;

	#[derive(Parser)]
	pub struct Args {}

	pub async fn list(_args: Args) -> Result<()> {
		todo!();
	}
}

pub mod create {
	#[allow(clippy::wildcard_imports)]
	use super::*;

	#[derive(Parser)]
	pub struct Args {}

	pub async fn create(_args: Args) -> Result<()> {
		todo!();
	}
}

pub mod delete {
	#[allow(clippy::wildcard_imports)]
	use super::*;

	#[derive(Parser)]
	pub struct Args {}

	pub async fn delete(_args: Args) -> Result<()> {
		todo!();
	}
}

pub mod start {
	#[allow(clippy::wildcard_imports)]
	use super::*;

	#[derive(Parser)]
	pub struct Args {}

	pub async fn start(_args: Args) -> Result<()> {
		todo!()
	}
}

pub mod stop {
	#[allow(clippy::wildcard_imports)]
	use super::*;

	#[derive(Parser)]
	pub struct Args {}

	pub async fn stop(_args: Args) -> Result<()> {
		todo!()
	}
}
