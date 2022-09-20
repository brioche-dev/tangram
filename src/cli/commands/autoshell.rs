use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use futures::FutureExt;
use std::path::PathBuf;
use tangram::client::Client;

#[derive(Parser)]
pub struct Autoshell {
	#[clap(subcommand)]
	subcommand: Subcommand,
}

#[derive(Parser)]
pub enum Subcommand {
	Add(AddArgs),
	Remove(RemoveArgs),
	List(ListArgs),
}

#[derive(Parser, Debug)]
pub struct AddArgs {
	path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct RemoveArgs {
	path: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct ListArgs {}

pub async fn run(args: Autoshell) -> Result<()> {
	match args.subcommand {
		Subcommand::Add(args) => add(args).boxed(),
		Subcommand::Remove(args) => remove(args).boxed(),
		Subcommand::List(_) => list().boxed(),
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
	let path = if let Some(path) = args.path {
		path
	} else {
		std::env::current_dir().context("Failed to determine the current directory.")?
	};

	// Add the autoshell.
	client.create_autoshell(&path).await?;

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
	let path = if let Some(path) = args.path {
		path
	} else {
		std::env::current_dir().context("Failed to determine the current directory.")?
	};

	// Remove the autoshell.
	client.delete_autoshell(&path).await?;

	Ok(())
}

async fn list() -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// List the autoshells
	let paths = client.get_autoshells().await?;

	println!("{paths:?}");

	Ok(())
}
