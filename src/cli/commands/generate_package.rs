use crate::config::Config;
use anyhow::{Context, Result};
use clap::Parser;
use futures::future::try_join_all;
use indoc::{formatdoc, indoc};
use tangram::client::Client;

#[derive(Parser)]
pub struct Args {
	#[clap(long, default_value = "new_package")]
	name: String,
	#[clap(long, default_value = "0.0.0")]
	version: String,
	#[clap(long)]
	output: Option<std::path::PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	// Read the config.
	let config = Config::read().await.context("Failed to read the config.")?;

	// Create the client.
	let client = Client::new_with_config(config.client)
		.await
		.context("Failed to create the client.")?;

	let Args { name, version, .. } = args;

	// Build the package artifact expression.
	let tangram_json_contents = formatdoc! {r#"
		{{
			"name": "{name}",
			"targets": ["default"],
			"version": "{version}",
			"dependencies": {{
				"std": {{ "path": "../std" }}
			}}
		}}"#};
	let tangram_ts_contents = indoc! {r#"
		import * as tg from "tangram:std/lib"

		export default async () => {
			return tg.artifact(
				tg.directory({
					hello: tg.file(
						tg.blob("Hello, Tangram!\n")
					)
				})
			)
		}"#};
	let tsconfig_contents = indoc! {r#"
		{
  			"compilerOptions": {
    			"moduleResolution": "node",
    			"paths": {
      				"tangram:std/*": ["../std/*"]
    			}
			},
  			"extends": "../../tsconfig.base.json"
		}"#};
	let files = [
		("tangram.json", tangram_json_contents.as_str()),
		("tangram.ts", tangram_ts_contents),
		("tsconfig.json", tsconfig_contents),
	];
	// Populate tempdir
	let tmpdir = tempfile::tempdir().context("Failed to create tmpdir.")?;
	let tmpdir_path = tmpdir.path();
	try_join_all(files.iter().map(|(filename, contents)| async move {
		let file_path = tmpdir_path.join(filename);
		tokio::fs::write(file_path, contents).await?;
		Ok::<_, anyhow::Error>(())
	}))
	.await?;

	// Check in tempdir
	let artifact = client
		.checkin(tmpdir_path)
		.await
		.context("Failed to check in package")?;
	println!("{artifact}");

	if let Some(output) = args.output {
		client
			.checkout(artifact, &output, None)
			.await
			.context("Failed to checkout package.")?;
		println!("New package created at {}", output.display());
	}

	Ok(())
}
