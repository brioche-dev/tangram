use anyhow::{Context, Result};
use clap::Parser;
use indoc::formatdoc;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Args {
	#[clap(long)]
	name: String,
	#[clap(long)]
	version: String,
	path: Option<PathBuf>,
}

pub async fn run(args: Args) -> Result<()> {
	// Resolve the path.
	let mut path =
		std::env::current_dir().context("Failed to get the current working directory.")?;
	if let Some(path_arg) = args.path {
		path.push(path_arg);
	}

	// Ensure the path exists.
	tokio::fs::create_dir_all(&path)
		.await
		.with_context(|| format!(r#"Failed to create the directory at "{}"."#, path.display()))?;

	// Get the package name and version.
	let name = args.name;
	let version = args.version;

	// Define the files to generate.
	let mut files = Vec::new();

	files.push((
		path.join("tangram.json"),
		formatdoc!(
			r#"
				{{
					"name": "{name}",
					"targets": ["default"],
					"version": "{version}",
					"dependencies": {{
						"std": {{ "path": "../std" }}
					}}
				}}
			"#,
		),
	));

	files.push((
		path.join("tangram.ts"),
		formatdoc!(
			r#"
				import * as tg from "tangram:std/lib";

				export default async () => {{
					return tg.artifact(
						tg.directory({{
							hello: tg.file(tg.blob("Hello, Tangram!\n")),
						}})
					);
				}};
			"#,
		),
	));

	files.push((
		path.join("tsconfig.json"),
		formatdoc!(
			r#"
				{{
					"compilerOptions": {{
						"moduleResolution": "node",
						"paths": {{
								"tangram:std/*": ["../std/*"]
						}}
					}},
					"extends": "../../tsconfig.base.json"
				}}
			"#
		),
	));

	// Write the files.
	for (path, contents) in files {
		tokio::fs::write(&path, &contents).await?;
	}

	Ok(())
}
