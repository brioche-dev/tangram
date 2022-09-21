use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::path::PathBuf;
use tangram::client::Client;

#[derive(Parser)]
pub struct Args {
	#[clap(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	Add(AddArgs),
	Hook(HookArgs),
	List(ListArgs),
	Remove(RemoveArgs),
}

#[derive(Parser, Debug)]
pub struct AddArgs {
	path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct ListArgs {}

#[derive(Parser, Debug)]
pub struct HookArgs {}

#[derive(Parser, Debug)]
pub struct RemoveArgs {
	path: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	match args.subcommand {
		Subcommand::Add(args) => add(args).boxed(),
		Subcommand::List(args) => list(args).boxed(),
		Subcommand::Hook(args) => hook(args).boxed(),
		Subcommand::Remove(args) => remove(args).boxed(),
	}
	.await?;
	Ok(())
}

async fn add(args: AddArgs) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Get the path.
	let mut path =
		std::env::current_dir().context("Failed to get the current working directory.")?;
	if let Some(path_arg) = args.path {
		path.push(path_arg);
	}
	let path = tokio::fs::canonicalize(&path)
		.await
		.context("Failed to canonicalize the path.")?;

	// Add the autoshell.
	client.create_autoshell(&path).await?;

	Ok(())
}

#[allow(clippy::unused_async)]
async fn hook(_args: HookArgs) -> Result<()> {
	todo!()
}

async fn list(_args: ListArgs) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// List the autoshells.
	let paths = client.get_autoshells().await?;

	println!("{paths:?}");

	Ok(())
}

async fn remove(args: RemoveArgs) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Get the path.
	let mut path =
		std::env::current_dir().context("Failed to get the current working directory.")?;
	if let Some(path_arg) = args.path {
		path.push(path_arg);
	}
	let path = tokio::fs::canonicalize(&path)
		.await
		.context("Failed to canonicalize the path.")?;

	// Remove the autoshell.
	client.delete_autoshell(&path).await?;

	Ok(())
}
