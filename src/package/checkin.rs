use super::Package;
use crate::{artifact::Artifact, directory, error::Result, path, util::fs};
use std::sync::Arc;

impl Package {
	/// Check in the package.
	pub async fn check_in(
		tg: &Arc<crate::instance::Instance>,
		package_path: &fs::Path,
	) -> Result<Self> {
		// Create a builder for the package directory.
		let mut directory = directory::Builder::new();

		// Add each module and its includes to the package directory.
		for (path, analyze_output) in Self::analyze_path(tg, package_path).await? {
			// Get the module's path.
			let module_path = package_path.join(path.to_string());

			// Add the module to the package directory.
			let artifact = Artifact::check_in(tg, &module_path).await?;
			directory = directory.add(tg, path.clone(), artifact).await?;

			// Add the includes to the package directory.
			for include_path in analyze_output.includes {
				// Get the included artifact's path in the package.
				let included_artifact_package_path = path
					.clone()
					.join(path::Component::Parent)
					.join(include_path.clone());

				// Get the included artifact's path.
				let included_artifact_path =
					package_path.join(included_artifact_package_path.to_string());

				// Check in the artifact at the included path.
				let included_artifact = Artifact::check_in(tg, &included_artifact_path).await?;

				// Add the included artifact to the directory.
				directory = directory
					.add(tg, included_artifact_package_path, included_artifact)
					.await?;
			}
		}

		// Create the package directory.
		let directory = directory.build(tg).await?;

		// Create the package.
		let package = Self::new(directory.into(), Some(package_path.to_owned()));

		Ok(package)
	}
}
