use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tangram::{
	artifact::Artifact, expression, fragment::Fragment, hash::Hash, object::ObjectHash,
	server::Server,
};

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = ".")]
	package: PathBuf,
	#[clap(long, default_value = "build")]
	name: String,
	#[clap(long, takes_value = false)]
	locked: bool,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	// let client = crate::client::new().await?;

	// // Checkin the package.
	// let package = client
	// 	.checkin_package(&args.package, args.locked)
	// 	.await
	// 	.context("Failed to check in package")?;

	// 	println!("Checked in package to artifact {package}");

	// // Evaluate the target.
	// let expression = tangram::expression::Expression::Target(tangram::expression::Target {
	// 	lockfile: None,
	// 	package,
	// 	name: args.name,
	// 	args: vec![],
	// });
	// let value = client.evaluate(expression).await?;

	// // Print the value.
	// let value = serde_json::to_string_pretty(&value)?;
	// println!("{value}");

	let server = Server::new("~/.tangram").await.unwrap();
	let artifact = "49b145f34deedbfd295e175abaa5674a8ba354f65e518e11cd9fd580d8bf2a9a".parse().unwrap();

	tokio::try_join!(
		server.create_fragment(artifact),
		server.create_fragment(artifact),
		server.create_fragment(artifact)
	).unwrap();

	Ok(())
}
