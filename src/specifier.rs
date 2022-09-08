use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
enum Specifier {
	Path(PathSpecifier),
	Registry(RegistrySpecifier),
	// Github(GithubSpecifier),
	// HttpTarball(HttpTarballSpecifier),
}

// A path to a folder on the file system where there is a tangram.json defining a Tangram package. A path must contain a "." so it can be disambiguated from a registry dependency.
// ./path/to/package
#[derive(Debug, PartialEq, Eq)]
struct PathSpecifier {
	path: PathBuf,
}

// The name of a Tangram package in configured registry.
// package_name (implicitly package_name@*)
// package_name@version
#[derive(Debug, PartialEq, Eq)]
struct RegistrySpecifier {
	package_name: String,
	version: Option<String>,
}

impl std::str::FromStr for Specifier {
	type Err = anyhow::Error;
	fn from_str(source: &str) -> Result<Specifier> {
		if source.starts_with('.') {
			// Parse this as a path specifier.
			let path = PathBuf::from_str(source)?;
			Ok(Specifier::Path(PathSpecifier { path }))
		} else {
			// Parse this as a registry specifier.
			let mut components = source.split('@');
			let package_name = components.next().unwrap().to_owned();
			let version = components.next().map(ToOwned::to_owned);
			Ok(Specifier::Registry(RegistrySpecifier {
				package_name,
				version,
			}))
		}
	}
}

// // A http endpoint where there is tarball that contains a Tangram package.
// https://tangram.dev/some/path/to/a/tarball/master.tar.gz
// struct HttpTarballSpecifier {
// 	url: Url,
// }

// // A github repo where there is a tangram.json defining a Tangram package.
// // github:tangramdotdev/tangram
// // github:tangramdotdev/tangram?ref={REF_NAME}
// // github:tangramdotdev/tangram?rev={COMMIT_HASH}
// struct GithubSpecifier {
// 	repo_name: String,
// 	repo_owner: String,
// 	reference: Option<String>,
// 	revision: Option<String>,
// }

#[cfg(test)]
mod tests {

	use super::*;

	#[test]
	fn test_parse_specifier() {
		let path_specifiers = ["./hello", "./", "."];
		for path_specifier in path_specifiers {
			let left: Specifier = path_specifier.parse().unwrap();
			let right = Specifier::Path(PathSpecifier {
				path: PathBuf::from(path_specifier),
			});
			assert_eq!(left, right);
		}

		let left: Specifier = "hello".parse().unwrap();
		let right = Specifier::Registry(RegistrySpecifier {
			package_name: "hello".to_owned(),
			version: None,
		});
		assert_eq!(left, right);

		let left: Specifier = "hello@version".parse().unwrap();
		let right = Specifier::Registry(RegistrySpecifier {
			package_name: "hello".to_owned(),
			version: Some("version".to_owned()),
		});
		assert_eq!(left, right);
	}
}
