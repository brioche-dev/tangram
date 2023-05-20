use super::{Package, ROOT_MODULE_FILE_NAME};
use crate::{
	error::{Result, WrapErr},
	module::{self, Module},
	path::Subpath,
};
use std::{
	collections::{HashMap, VecDeque},
	path::Path,
	sync::Arc,
};

impl Package {
	pub(crate) async fn analyze_path(
		package_path: &Path,
	) -> Result<HashMap<Subpath, module::analyze::Output, fnv::FnvBuildHasher>> {
		// Create a queue of paths to visit and a visited set.
		let mut output = HashMap::default();
		let mut queue = VecDeque::from(vec![Subpath::try_from(ROOT_MODULE_FILE_NAME).unwrap()]);

		while let Some(path) = queue.pop_front() {
			// Get the module's text.
			let module_path = package_path.join(path.to_string());
			let text = tokio::fs::read_to_string(&module_path)
				.await
				.wrap_err("Failed to read the module.")?;

			// Analyze the module and get its imports.
			let analyze_output = Module::analyze(text).wrap_err("Failed to analyze the module.")?;

			// Add the path and analyze output to the output.
			output.insert(path.clone(), analyze_output.clone());

			// Add the unvisited import paths to the queue.
			for specifier in &analyze_output.imports {
				if let module::Specifier::Path(specifier) = specifier {
					let path = path
						.clone()
						.into_relpath()
						.parent()
						.join(specifier.clone())
						.try_into_subpath()
						.wrap_err("Failed to resolve the module path.")?;
					if !output.contains_key(&path) {
						queue.push_back(path);
					}
				}
			}
		}

		Ok(output)
	}

	pub(crate) async fn analyze(
		&self,
		tg: &Arc<crate::instance::Instance>,
	) -> Result<HashMap<Subpath, module::analyze::Output, fnv::FnvBuildHasher>> {
		// Create a queue of paths to visit and a visited set.
		let mut output = HashMap::default();
		let mut queue = VecDeque::from(vec![Subpath::try_from(ROOT_MODULE_FILE_NAME).unwrap()]);

		while let Some(path) = queue.pop_front() {
			// Get the module's text.
			let text = self
				.artifact()
				.as_directory()
				.unwrap()
				.get(tg, &path.clone())
				.await?
				.into_file()
				.unwrap()
				.blob()
				.text(tg)
				.await?;

			// Analyze the module and get its imports.
			let analyze_output = Module::analyze(text)?;

			// Add the path and analyze output to the output.
			output.insert(path.clone(), analyze_output.clone());

			// Add the unvisited import paths to the queue.
			for specifier in &analyze_output.imports {
				if let module::Specifier::Path(specifier) = specifier {
					let path = path
						.clone()
						.into_relpath()
						.parent()
						.join(specifier.clone())
						.try_into_subpath()
						.wrap_err("Invalid specifier.")?;
					if !output.contains_key(&path) {
						queue.push_back(path);
					}
				}
			}
		}

		Ok(output)
	}
}
