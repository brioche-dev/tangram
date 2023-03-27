use crate::{
	error::{return_error, Result, WrapErr},
	Cli,
};
use indoc::formatdoc;
use tangram::util::fs;

/// Initialize a new package.
#[derive(Debug, clap::Args)]
pub struct Args {
	#[arg(long)]
	pub name: Option<String>,

	#[arg(long)]
	pub version: Option<String>,

	pub path: Option<fs::PathBuf>,
}

impl Cli {
	pub async fn command_init(&self, args: Args) -> Result<()> {
		// Get the path.
		let mut path =
			std::env::current_dir().wrap_err("Failed to get the current working directory.")?;
		if let Some(path_arg) = &args.path {
			path.push(path_arg);
		}

		// Ensure there is a directory at the path.
		match tokio::fs::metadata(&path).await {
			Ok(metadata) => {
				if !metadata.is_dir() {
					return_error!("The path must be a directory.");
				}
			},
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
				tokio::fs::create_dir_all(&path).await.wrap_err_with(|| {
					let path = path.display();
					format!(r#"Failed to create the directory at "{path}"."#)
				})?;
			},
			Err(error) => return Err(error.into()),
		};

		// Get the package name. The default package name is the last component of the path.
		let _name = if let Some(name) = args.name {
			name
		} else {
			let canonicalized_path = tokio::fs::canonicalize(&path).await?;
			let last_path_component = canonicalized_path.components().last().unwrap();
			let last_path_component_string = last_path_component
				.as_os_str()
				.to_str()
				.wrap_err("The last component of the path must be valid UTF-8.")?;
			last_path_component_string.to_owned()
		};

		// Get the version. The default version is 0.0.0.
		let _version = args.version.unwrap_or_else(|| String::from("0.0.0"));

		// Define the files to generate.
		let mut files = Vec::new();

		files.push((
			path.join("tangram.tg"),
			formatdoc!(
				r#"
					// Write your code here.
				"#,
			),
		));

		// Write the files.
		for (path, contents) in files {
			tokio::fs::write(&path, &contents).await?;
		}

		Ok(())
	}
}
