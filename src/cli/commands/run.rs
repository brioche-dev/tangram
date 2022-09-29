use anyhow::{anyhow, bail, Context, Result};
use clap::Parser;
use std::path::PathBuf;
use tangram::{
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
	#[clap(long)]
	pub target: Option<String>,
	#[clap(default_value = ".")]
	pub specifier: Specifier,
	pub trailing_args: Vec<String>,
}

pub async fn run(args: Args) -> Result<()> {
	// Create the client.
	let client = crate::client::new().await?;

	// Get the package hash.
	let package_hash = match args.specifier {
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
				.ok_or_else(|| {
					anyhow!(
						r#"Could not find version "{version}" of the package "{package_name}"."#
					)
				})?
		},
	};

	// Get the client's in process server.
	let server = match client.as_in_process() {
		Some(server) => server,
		None => {
			bail!("Client must be connected to an in process server in order to run.")
		},
	};

	// Get the package manifest.
	let manifest = server.get_package_manifest(package_hash).await?;

	// Get the package name.
	let package_name = manifest.name;

	// Get the executable path.
	let executable_path = args
		.executable_path
		.unwrap_or_else(|| PathBuf::from("bin").join(package_name));

	// Get the target name.
	let name = args.target.unwrap_or_else(|| "default".to_owned());

	// Add the args.
	let target_args = client
		.add_expression(&tangram::expression::Expression::Array(vec![]))
		.await?;

	// Create the expression.
	let input_hash = client
		.add_expression(&tangram::expression::Expression::Target(
			tangram::expression::Target {
				lockfile: None,
				package: package_hash,
				name,
				args: target_args,
			},
		))
		.await?;

	// Evaluate the expression.
	let output_hash = client
		.evaluate(input_hash)
		.await
		.context("Failed to evaluate the target expression.")?;

	// Check that the client is connected to an in-process server.
	let server = match client.as_in_process() {
		Some(server) => server,
		None => {
			bail!("The client must use the in process transport.");
		},
	};

	// Create a fragment for the output.
	let fragment = server.create_fragment(output_hash).await?;

	// Get the path to the fragment.
	let path = server.fragment_path(&fragment);

	// Get the path to the executable.
	let executable_path = path.join(executable_path);

	// Run the process!
	let mut child = tokio::process::Command::new(&executable_path)
		.args(args.trailing_args)
		.spawn()?;
	child.wait().await?;

	Ok(())
}
