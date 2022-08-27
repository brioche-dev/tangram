use camino::Utf8PathBuf;
use semver::{Version, VersionReq};
use std::collections::BTreeMap;
use url::Url;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
	pub name: String,
	pub version: Version,
	pub targets: Vec<String>,
	pub dependencies: Option<BTreeMap<String, Dependency>>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(from = "DependencySerde", into = "DependencySerde")]
pub enum Dependency {
	PathDependency(PathDependency),
	RegistryDependency(RegistryDependency),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PathDependency {
	pub name: Option<String>,
	pub path: Utf8PathBuf,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct RegistryDependency {
	pub registry: Option<Url>,
	pub name: Option<String>,
	pub version: VersionReq,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DependencySerde {
	VersionReq(VersionReq),
	Path(PathDependency),
	Registry(RegistryDependency),
}

impl From<Dependency> for DependencySerde {
	fn from(value: Dependency) -> Self {
		match value {
			Dependency::PathDependency(dependency) => DependencySerde::Path(dependency),
			Dependency::RegistryDependency(dependency) => DependencySerde::Registry(dependency),
		}
	}
}

impl From<DependencySerde> for Dependency {
	fn from(value: DependencySerde) -> Self {
		match value {
			DependencySerde::VersionReq(string) => {
				Dependency::RegistryDependency(RegistryDependency {
					name: None,
					registry: None,
					version: string,
				})
			},
			DependencySerde::Path(dependency) => Dependency::PathDependency(dependency),
			DependencySerde::Registry(dependency) => Dependency::RegistryDependency(dependency),
		}
	}
}
