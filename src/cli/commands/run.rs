use crate::config::Config;
use anyhow::{bail, Context, Result};
use clap::Parser;
use tangram::{
	client::Client,
	specifier::{PathSpecifier, RegistrySpecifier, Specifier},
};

#[derive(Parser, Debug)]
pub struct Args {
	#[clap(long)]
	executable_path: Option<String>,
	#[clap(long, takes_value = false)]
	locked: bool,
	#[clap(long, default_value = "build")]
	name: String,
	#[clap(default_value = ".")]
	package: String,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	// Parse the package specifier.
	let package_specifier: Specifier = args
		.package
		.parse()
		.context("Failed to parse the package specifier.")?;

	// Evaluate and checkout the resulting artifact.
	let artifact = match package_specifier {
		Specifier::Path(PathSpecifier { path }) => {
			// Checkin the package.
			let package = client
				.checkin_package(&path, args.locked)
				.await
				.context("Failed to check in the package.")?;

			// Create the expression.
			let expression = tangram::expression::Expression::Target(tangram::expression::Target {
				lockfile: None,
				package,
				name: args.name.clone(),
				args: vec![],
			});

			// Evaluate the expression.
			let value = client
				.evaluate(expression)
				.await
				.context("Failed to evaluate the target expression.")?;

			match value {
				tangram::value::Value::Artifact(artifact) => artifact,
				_ => bail!("Failed to run. The provided target must evaluate to an artifact in order to be run."),
			}
		},
		Specifier::Registry(RegistrySpecifier { .. }) => {
			todo!()
		},
	};

	// Check that the client is connected to an in-process server.
	let server = match client.as_in_process() {
		Some(server) => server,
		None => {
			bail!("Client must be connected to an 'In Process' Server in order to run.")
		},
	};

	// Create a fragment for the artifact.
	let fragment = server.create_fragment(artifact).await?;

	// Get the path to the fragment.
	let path = server.fragment_path(&fragment);

	// Get the path to the executable.
	let executable_path = if let Some(executable_path) = args.executable_path {
		path.join(executable_path)
	} else {
		path.join("bin").join(&args.name)
	};

	// Run the process!
	let mut child = tokio::process::Command::new(&executable_path).spawn()?;
	child.wait().await?;

	Ok(())
}
