use crate::config::Config;
use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::{
	client::Client,
	manifest::Manifest,
	specifier::{self, Specifier},
};

#[derive(Parser, Debug)]
#[clap(trailing_var_arg = true)]
pub struct Args {
	#[clap(long)]
	pub executable_path: Option<PathBuf>,
	#[clap(long, takes_value = false)]
	pub locked: bool,
	#[clap(long, default_value = "build")]
	pub name: String,
	#[clap(default_value = ".")]
	pub package: String,
	pub trailing_args: Vec<String>,
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

	// Get the package artifact.
	let package = match package_specifier {
		Specifier::Path(specifier::Path { path }) => {
			// Checkin the package.
			client
				.checkin_package(&path, args.locked)
				.await
				.context("Failed to check in the package.")?
		},
		Specifier::Registry(specifier::Registry {
			package_name,
			version,
		}) => {
			// Get the package from the registry.
			let version = version.ok_or_else(|| anyhow!("A version is required."))?;
			client
				.get_package(&package_name, &version)
				.await
				.with_context(|| {
					format!(r#"Failed to get the package "{package_name}" from the registry."#)
				})?
				.ok_or_else(|| anyhow!(r#"Failed to get the package "{package_name}"."#))?
		},
	};

	// Check that the client is connected to an in-process server.
	let server = match client.as_in_process() {
		Some(server) => server,
		None => {
			bail!("Client must be connected to an 'In Process' Server in order to run.")
		},
	};

	// Create a fragment for the package.
	let package_fragment = server.create_fragment(package).await?;

	// Get the path to the fragment.
	let package_path = server.fragment_path(&package_fragment);

	// Materialize the package to read the manifest.
	let manifest_path = package_path.join("tangram.json");
	let manifest = tokio::fs::read(&manifest_path)
		.await
		.context("Failed to read the package manifest.")?;
	let manifest: Manifest = serde_json::from_slice(&manifest).with_context(|| {
		format!(
			r#"Failed to parse the package manifest at path "{}"."#,
			manifest_path.display()
		)
	})?;

	let executable_subpath = if let Some(executable_subpath) = args.executable_path {
		executable_subpath
	} else {
		PathBuf::from("bin").join(manifest.name)
	};

	// Create the expression.
	let expression = tangram::expression::Expression::Target(tangram::expression::Target {
		lockfile: None,
		package,
		name: args.name,
		args: vec![],
	});

	// Evaluate the expression.
	let output = client
		.evaluate(&expression)
		.await
		.context("Failed to evaluate the target expression.")?;

	let artifact = match output {
		tangram::expression::Expression::Artifact(artifact) => artifact,
		_ => bail!("The target must evaluate to an artifact."),
	};

	// Check that the client is connected to an in-process server.
	let server = match client.as_in_process() {
		Some(server) => server,
		None => {
			bail!("The client must use the in process transport.");
		},
	};

	// Create a fragment for the artifact.
	let fragment = server.create_fragment(artifact).await?;

	// Get the path to the fragment.
	let path = server.fragment_path(&fragment);

	// Get the path to the executable.
	let executable_path = path.join(executable_subpath);

	// Run the process!
	let mut child = tokio::process::Command::new(&executable_path)
		.args(args.trailing_args)
		.spawn()?;
	child.wait().await?;

	Ok(())
}
