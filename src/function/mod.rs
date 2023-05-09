pub use self::data::Data;
use crate::{error::Result, instance::Instance, package, path::Subpath};

mod data;

/// A function.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
	/// The hash of the package instance of the function.
	pub package_instance_hash: package::instance::Hash,

	/// The path to module where the function is defined.
	pub module_path: Subpath,

	/// The name of the function.
	pub name: String,
}

impl Function {
	#[must_use]
	pub fn new(package_instance: &package::Instance, module_path: Subpath, name: String) -> Self {
		Self {
			package_instance_hash: package_instance.hash(),
			module_path,
			name,
		}
	}

	pub async fn package_instance(&self, tg: &Instance) -> Result<package::Instance> {
		package::Instance::get(tg, self.package_instance_hash).await
	}
}
