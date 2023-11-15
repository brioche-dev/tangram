use super::{version::solve, ROOT_MODULE_FILE_NAME};
use async_trait::async_trait;
use futures::stream::BoxStream;
use tangram_client as tg;
use tangram_error::WrapErr;
use tg::Client;

#[tokio::test]
async fn simple_diamond() {
	let client = MockClient::new().await;
	client
		.create_mock_package(
			"simple_diamond_A",
			"1.0.0",
			&[
				tg::Dependency::with_name_and_version(
					"simple_diamond_B".into(),
					Some("^1.0".into()),
				),
				tg::Dependency::with_name_and_version(
					"simple_diamond_C".into(),
					Some("^1.0".into()),
				),
			],
		)
		.await;
	client
		.create_mock_package(
			"simple_diamond_B",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"simple_diamond_D".into(),
				Some("^1.0".into()),
			)],
		)
		.await;

	client
		.create_mock_package(
			"simple_diamond_C",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"simple_diamond_D".into(),
				Some("^1.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package("simple_diamond_D", "1.0.0", &[])
		.await;

	let metadata = tg::package::Metadata {
		name: Some("simple_diamond_A".into()),
		version: Some("1.0.0".into()),
		description: None,
	};

	let _lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve simple_diamond case.");
}

#[tokio::test]
async fn simple_backtrack() {
	let client = MockClient::new().await;
	client
		.create_mock_package(
			"simple_backtrack_A",
			"1.0.0",
			&[
				tg::Dependency::with_name_and_version(
					"simple_backtrack_B".into(),
					Some("^1.2.3".into()),
				),
				tg::Dependency::with_name_and_version(
					"simple_backtrack_C".into(),
					Some("<1.2.3".into()),
				),
			],
		)
		.await;
	client
		.create_mock_package(
			"simple_backtrack_B",
			"1.2.3",
			&[tg::Dependency::with_name_and_version(
				"simple_backtrack_C".into(),
				Some("<1.2.3".into()),
			)],
		)
		.await;
	client
		.create_mock_package("simple_backtrack_C", "1.2.3", &[])
		.await;
	client
		.create_mock_package("simple_backtrack_C", "1.2.2", &[])
		.await;

	let metadata = tg::package::Metadata {
		name: Some("simple_backtrack_A".into()),
		version: Some("1.0.0".into()),
		description: None,
	};

	let _lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve simple_backtrack case.");
}

#[tokio::test]
async fn diamond_backtrack() {
	let client = MockClient::new().await;
	client
		.create_mock_package(
			"diamond_backtrack_A",
			"1.0.0",
			&[
				tg::Dependency::with_name_and_version(
					"diamond_backtrack_B".into(),
					Some("1.0.0".into()),
				),
				tg::Dependency::with_name_and_version(
					"diamond_backtrack_C".into(),
					Some("1.0.0".into()),
				),
			],
		)
		.await;
	client
		.create_mock_package(
			"diamond_backtrack_B",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"diamond_backtrack_D".into(),
				Some("<1.5.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package(
			"diamond_backtrack_C",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"diamond_backtrack_D".into(),
				Some("<1.3.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package("diamond_backtrack_D", "1.1.0", &[])
		.await;
	client
		.create_mock_package("diamond_backtrack_D", "1.2.0", &[])
		.await;
	client
		.create_mock_package("diamond_backtrack_D", "1.3.0", &[])
		.await;
	client
		.create_mock_package("diamond_backtrack_D", "1.4.0", &[])
		.await;
	client
		.create_mock_package("diamond_backtrack_D", "1.5.0", &[])
		.await;

	let metadata: tg::package::Metadata = tg::package::Metadata {
		name: Some("diamond_backtrack_A".into()),
		version: Some("1.0.0".into()),
		description: None,
	};

	let _lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve diamond_backtrack case.");
}

#[tokio::test]
async fn cycle_exists() {
	let client = MockClient::new().await;
	client
		.create_mock_package(
			"cycle_exists_A",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"cycle_exists_B".into(),
				Some("1.0.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package(
			"cycle_exists_B",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"cycle_exists_C".into(),
				Some("1.0.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package(
			"cycle_exists_C",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"cycle_exists_B".into(),
				Some("1.0.0".into()),
			)],
		)
		.await;

	let metadata: tg::package::Metadata = tg::package::Metadata {
		name: Some("cycle_exists_A".into()),
		version: Some("1.0.0".into()),
		description: None,
	};

	let report = client
		.try_solve(metadata.clone())
		.await
		.expect_err("Expected to fail with cycle detection.");

	println!("{report}");
}

#[tokio::test]
async fn diamond_incompatible_versions() {
	let client = MockClient::new().await;
	client
		.create_mock_package(
			"diamond_incompatible_versions_A",
			"1.0.0",
			&[
				tg::Dependency::with_name_and_version(
					"diamond_incompatible_versions_B".into(),
					Some("1.0.0".into()),
				),
				tg::Dependency::with_name_and_version(
					"diamond_incompatible_versions_C".into(),
					Some("1.0.0".into()),
				),
			],
		)
		.await;
	client
		.create_mock_package(
			"diamond_incompatible_versions_B",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"diamond_incompatible_versions_D".into(),
				Some("<1.2.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package(
			"diamond_incompatible_versions_C",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"diamond_incompatible_versions_D".into(),
				Some(">1.3.0".into()),
			)],
		)
		.await;

	client
		.create_mock_package("diamond_incompatible_versions_D", "1.0.0", &[])
		.await;
	client
		.create_mock_package("diamond_incompatible_versions_D", "1.1.0", &[])
		.await;
	client
		.create_mock_package("diamond_incompatible_versions_D", "1.2.0", &[])
		.await;
	client
		.create_mock_package("diamond_incompatible_versions_D", "1.3.0", &[])
		.await;
	client
		.create_mock_package("diamond_incompatible_versions_D", "1.4.0", &[])
		.await;

	let metadata = tg::package::Metadata {
		name: Some("diamond_incompatible_versions_A".into()),
		version: Some("1.0.0".into()),
		description: None,
	};
	let report = client
		.try_solve(metadata.clone())
		.await
		.expect_err("Expected to fail with cycle detection.");

	println!("{report}");
}

#[tokio::test]
#[allow(clippy::similar_names)]
async fn diamond_with_path_dependencies() {
	let foo = r#"
		export let metadata = {
			name: "foo",
			version: "1.0.0",
		};

		import bar from "tangram:?path=./path/to/bar";
		import baz from "tangram:baz@^1";
		export default tg.target(() => tg`foo ${bar} {baz}`);
	"#;

	let bar = r#"
		export let metadata = {
			name: "bar",
			version: "1.0.0",
		};

		import * as baz from "tangram:baz@=1.2.3";
		export default tg.target(() => tg`bar ${baz}`);
	"#;

	let baz = r#"
		export let metadata = {
			name: "baz",
			version: "1.2.3",
		};

		export default tg.target(() => "baz");
	"#;
	let client = MockClient::new().await;

	let foo = tg::Directory::new(
		[(
			ROOT_MODULE_FILE_NAME.into(),
			tg::Artifact::from(
				tg::File::builder(
					tg::blob::Blob::with_reader(&client, foo.as_bytes())
						.await
						.unwrap(),
				)
				.build(),
			),
		)]
		.into_iter()
		.collect(),
	);

	let bar = tg::Directory::new(
		[(
			ROOT_MODULE_FILE_NAME.into(),
			tg::Artifact::from(
				tg::File::builder(
					tg::blob::Blob::with_reader(&client, bar.as_bytes())
						.await
						.unwrap(),
				)
				.build(),
			),
		)]
		.into_iter()
		.collect(),
	);

	let baz = tg::Directory::new(
		[(
			ROOT_MODULE_FILE_NAME.into(),
			tg::Artifact::from(
				tg::File::builder(
					tg::blob::Blob::with_reader(&client, baz.as_bytes())
						.await
						.unwrap(),
				)
				.build(),
			),
		)]
		.into_iter()
		.collect(),
	);
	client
		.publish_package("", &baz.id(&client).await.unwrap().clone().into())
		.await
		.unwrap();

	// Create the path dependency table.
	let foo_id: tg::Id = foo.id(&client).await.unwrap().clone().into();
	let foo_path_dependencies: BTreeMap<tg::Relpath, tg::Id> = [(
		"./path/to/bar".parse().unwrap(),
		bar.id(&client).await.unwrap().clone().into(),
	)]
	.into_iter()
	.collect();
	let path_dependencies = [(foo_id.clone(), foo_path_dependencies)]
		.into_iter()
		.collect();

	// Create the registry dependencies
	let bar_id = bar.id(&client).await.unwrap().clone().into();
	let baz_id = baz.id(&client).await.unwrap().clone().into();
	let registry_dependencies = client
		.registry_dependencies(&[foo_id.clone(), bar_id, baz_id])
		.await;

	// Lock using foo as the root.
	let _lock = solve(&client, foo_id, path_dependencies, registry_dependencies)
		.await
		.expect("Failed to lock diamond with a path dependency.");
}

#[tokio::test]
async fn complex_diamond() {
	let client = MockClient::new().await;
	client
		.create_mock_package(
			"complex_diamond_A",
			"1.0.0",
			&[
				tg::Dependency::with_name_and_version(
					"complex_diamond_B".into(),
					Some("^1.0.0".into()),
				),
				tg::Dependency::with_name_and_version(
					"complex_diamond_E".into(),
					Some("^1.1.0".into()),
				),
				tg::Dependency::with_name_and_version(
					"complex_diamond_C".into(),
					Some("^1.0.0".into()),
				),
				tg::Dependency::with_name_and_version(
					"complex_diamond_D".into(),
					Some("^1.0.0".into()),
				),
			],
		)
		.await;
	client
		.create_mock_package(
			"complex_diamond_B",
			"1.0.0",
			&[tg::Dependency::with_name_and_version(
				"complex_diamond_D".into(),
				Some("^1.0.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package(
			"complex_diamond_C",
			"1.0.0",
			&[
				tg::Dependency::with_name_and_version(
					"complex_diamond_D".into(),
					Some("^1.0.0".into()),
				),
				tg::Dependency::with_name_and_version(
					"complex_diamond_E".into(),
					Some(">1.0.0".into()),
				),
			],
		)
		.await;

	client
		.create_mock_package(
			"complex_diamond_D",
			"1.3.0",
			&[tg::Dependency::with_name_and_version(
				"complex_diamond_E".into(),
				Some("=1.0.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package(
			"complex_diamond_D",
			"1.2.0",
			&[tg::Dependency::with_name_and_version(
				"complex_diamond_E".into(),
				Some("^1.0.0".into()),
			)],
		)
		.await;
	client
		.create_mock_package("complex_diamond_E", "1.0.0", &[])
		.await;
	client
		.create_mock_package("complex_diamond_E", "1.1.0", &[])
		.await;
	let metadata = tg::package::Metadata {
		name: Some("complex_diamond_A".into()),
		version: Some("1.0.0".into()),
		description: None,
	};
	let _lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve diamond_backtrack case.");
}

use std::{
	collections::{BTreeMap, BTreeSet, HashMap},
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

/// A test client for debugging the version solving algorithm without requiring a full backend.
#[derive(Clone)]
pub struct MockClient {
	state: Arc<Mutex<State>>,
	client: Arc<dyn tg::Client>,
}

#[derive(Debug)]
struct State {
	packages: BTreeMap<String, Vec<MockPackage>>,
	dependencies: HashMap<tg::package::Metadata, Vec<tg::Dependency>>,
}

#[derive(Debug)]
struct MockPackage {
	metadata: tg::package::Metadata,
	artifact: tg::Artifact,
}

impl MockClient {
	pub async fn new() -> Self {
		#[allow(clippy::map_unwrap_or)]
		let path = std::env::var("TANGRAM_PATH")
			.map(PathBuf::from)
			.unwrap_or_else(|_| {
				let home = PathBuf::from(std::env::var("HOME").unwrap());
				home.join(".tangram")
			});

		// Attempt to connect to the server.
		let addr = tangram_http::net::Addr::Unix(path.join("socket"));
		let client = tangram_http::client::Builder::new(addr).build();
		client.connect().await.unwrap();

		let state = State {
			packages: BTreeMap::new(),
			dependencies: HashMap::new(),
		};

		Self {
			client: Arc::new(client),
			state: Arc::new(Mutex::new(state)),
		}
	}

	pub fn publish(&self, metadata: tg::package::Metadata, artifact: tg::Artifact) {
		assert!(metadata.name.is_some());
		assert!(metadata.version.is_some());
		let mut state = self.state.lock().unwrap();
		let entry = state
			.packages
			.entry(metadata.name.as_ref().unwrap().clone())
			.or_default();
		let existing = entry.iter().any(|package| package.metadata == metadata);
		assert!(!existing);
		entry.push(MockPackage { metadata, artifact });
	}

	pub async fn create_mock_package(
		&self,
		name: &str,
		version: &str,
		dependencies: &[tg::Dependency],
	) {
		let imports = dependencies
			.iter()
			.map(|dep| {
				format!(
					r#"import * as {} from "tangram:{}@{}"#,
					dep.name.as_ref().unwrap(),
					dep.name.as_ref().unwrap(),
					dep.version.as_ref().unwrap()
				)
			})
			.collect::<Vec<_>>()
			.join("\n");

		let contents = format!(
			r#"
        {imports}
        export let metadata = {{
            name: "{name}",
            version: "{version}",
        }};

        export default tg.target(() => "Hello, from {name}!");
        "#
		);

		let contents = tg::blob::Blob::with_reader(self.client.as_ref(), contents.as_bytes())
			.await
			.unwrap();
		let tangram_tg = tg::Artifact::from(tg::File::builder(contents).build());
		let artifact = tg::Directory::with_object(tg::directory::Object {
			entries: [(ROOT_MODULE_FILE_NAME.to_owned(), tangram_tg)]
				.into_iter()
				.collect(),
		})
		.into();

		let metadata = tg::package::Metadata {
			name: Some(name.into()),
			version: Some(version.into()),
			description: None,
		};

		{
			let mut state = self.state.lock().unwrap();
			let _ = state
				.dependencies
				.insert(metadata.clone(), dependencies.to_vec());
		}
		self.publish(metadata, artifact);
	}

	pub async fn try_solve(
		&self,
		metadata: tg::package::Metadata,
	) -> tangram_error::Result<Vec<(tg::Relpath, tg::Lock)>> {
		let package: tg::Id = self
			.get_package_version(
				metadata.name.as_ref().unwrap(),
				metadata.version.as_ref().unwrap(),
			)
			.await
			.unwrap()
			.unwrap()
			.into();
		let registry_dependencies = self.registry_dependencies(&[package.clone()]).await;
		solve(self, package, BTreeMap::new(), registry_dependencies).await
	}

	pub async fn registry_dependencies(
		&self,
		packages: &[tg::Id],
	) -> BTreeSet<(tg::Id, tg::Dependency)> {
		let mut set = BTreeSet::new();
		for package in packages {
			let dependencies = self
				.get_package_dependencies(package)
				.await
				.expect("Failed to get the package dependencies.")
				.into_iter()
				.flatten()
				.filter_map(|d| d.path.is_none().then_some((package.clone(), d)));
			set.extend(dependencies);
		}
		set
	}
}

#[async_trait]
impl tg::Client for MockClient {
	fn clone_box(&self) -> Box<dyn tg::Client> {
		Box::new(self.clone())
	}

	fn path(&self) -> Option<&Path> {
		self.client.path()
	}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		self.client.file_descriptor_semaphore()
	}

	async fn stop(&self) -> tangram_error::Result<()> {
		self.client.stop().await
	}

	async fn status(&self) -> tangram_error::Result<tg::status::Status> {
		self.client.status().await
	}

	async fn clean(&self) -> tangram_error::Result<()> {
		self.client.clean().await
	}

	async fn get_object_exists(&self, id: &tg::object::Id) -> tangram_error::Result<bool> {
		self.client.get_object_exists(id).await
	}

	async fn try_get_object(
		&self,
		id: &tg::object::Id,
	) -> tangram_error::Result<Option<bytes::Bytes>> {
		self.client.try_get_object(id).await
	}

	async fn try_put_object(
		&self,
		id: &tg::object::Id,
		bytes: &bytes::Bytes,
	) -> tangram_error::Result<tangram_error::Result<(), Vec<tg::object::Id>>> {
		self.client.try_put_object(id, bytes).await
	}

	async fn try_get_tracker(&self, path: &Path) -> tangram_error::Result<Option<tg::Tracker>> {
		self.client.try_get_tracker(path).await
	}

	async fn set_tracker(&self, path: &Path, tracker: &tg::Tracker) -> tangram_error::Result<()> {
		self.client.set_tracker(path, tracker).await
	}

	async fn try_get_build_for_target(
		&self,
		id: &tg::target::Id,
	) -> tangram_error::Result<Option<tg::build::Id>> {
		self.client.try_get_build_for_target(id).await
	}

	async fn get_or_create_build_for_target(
		&self,
		id: &tg::target::Id,
	) -> tangram_error::Result<tg::build::Id> {
		self.client.get_or_create_build_for_target(id).await
	}

	async fn try_get_build_target(
		&self,
		id: &tg::build::Id,
	) -> tangram_error::Result<Option<tg::target::Id>> {
		self.client.try_get_build_target(id).await
	}

	async fn try_get_build_children(
		&self,
		id: &tg::build::Id,
	) -> tangram_error::Result<Option<BoxStream<'static, tangram_error::Result<tg::build::Id>>>> {
		self.client.try_get_build_children(id).await
	}

	async fn add_build_child(
		&self,
		build_id: &tg::build::Id,
		child_id: &tg::build::Id,
	) -> tangram_error::Result<()> {
		self.client.add_build_child(build_id, child_id).await
	}

	async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> tangram_error::Result<Option<BoxStream<'static, tangram_error::Result<bytes::Bytes>>>> {
		self.client.try_get_build_log(id).await
	}

	async fn add_build_log(
		&self,
		build_id: &tg::build::Id,
		bytes: bytes::Bytes,
	) -> tangram_error::Result<()> {
		self.client.add_build_log(build_id, bytes).await
	}

	async fn try_get_build_result(
		&self,
		id: &tg::build::Id,
	) -> tangram_error::Result<Option<tangram_error::Result<tg::Value>>> {
		self.client.try_get_build_result(id).await
	}

	async fn finish_build(
		&self,
		id: &tg::build::Id,
		result: tangram_error::Result<tg::Value>,
	) -> tangram_error::Result<()> {
		self.client.finish_build(id, result).await
	}

	async fn create_login(&self) -> tangram_error::Result<tg::user::Login> {
		self.client.create_login().await
	}

	async fn get_login(&self, id: &tg::Id) -> tangram_error::Result<Option<tg::user::Login>> {
		self.client.get_login(id).await
	}

	async fn get_current_user(&self, token: &str) -> tangram_error::Result<Option<tg::user::User>> {
		self.client.get_current_user(token).await
	}

	async fn search_packages(
		&self,
		quer: &str,
	) -> tangram_error::Result<Vec<tg::package::Package>> {
		let Some(mock) = self.get_package(quer).await? else {
			return Ok(vec![]);
		};
		Ok(vec![mock])
	}

	async fn get_package(&self, name: &str) -> tangram_error::Result<Option<tg::package::Package>> {
		let state = self.state.lock().unwrap();
		let Some(mock) = state.packages.get(name) else {
			return Ok(None);
		};

		let versions = mock
			.iter()
			.map(|mock| mock.metadata.version.clone().unwrap())
			.collect();
		Ok(Some(tg::Package {
			name: name.into(),
			versions,
		}))
	}

	async fn get_package_version(
		&self,
		name: &str,
		version: &str,
	) -> tangram_error::Result<Option<tg::artifact::Id>> {
		let artifact = {
			let state = self.state.lock().unwrap();
			let Some(mock) = state.packages.get(name) else {
				return Ok(None);
			};

			let Some(artifact) = mock.iter().find_map(|mock| {
				(mock.metadata.version.as_deref().unwrap() == version)
					.then_some(mock.artifact.clone())
			}) else {
				return Ok(None);
			};
			artifact
		};

		let id = artifact.id(self.client.as_ref()).await?;
		Ok(Some(id))
	}

	async fn publish_package(
		&self,
		_token: &str,
		id: &tg::artifact::Id,
	) -> tangram_error::Result<()> {
		let directory = tg::Artifact::with_id(id.clone())
			.try_unwrap_directory()
			.wrap_err("Failed to get directory")?;

		let subpath: tg::Subpath = ROOT_MODULE_FILE_NAME.parse().unwrap();
		let text = directory
			.try_get(self.client.as_ref(), &subpath)
			.await?
			.unwrap()
			.try_unwrap_file()
			.wrap_err("Failed to get the file.")?
			.contents(self.client.as_ref())
			.await?
			.text(self.client.as_ref())
			.await?;

		let module = crate::Module::analyze(text)?;
		let metadata = module.metadata.unwrap();

		self.publish(metadata, tg::Artifact::with_id(id.clone()));
		Ok(())
	}

	async fn get_package_metadata(
		&self,
		id: &tg::Id,
	) -> tangram_error::Result<Option<tg::package::Metadata>> {
		self.client.get_package_metadata(id).await
	}

	async fn get_package_dependencies(
		&self,
		id: &tg::Id,
	) -> tangram_error::Result<Option<Vec<tg::Dependency>>> {
		self.client.get_package_dependencies(id).await
	}

	async fn get_build_from_queue(&self) -> tangram_error::Result<tg::build::Id> {
		self.client.get_build_from_queue().await
	}

	async fn cancel_build(&self, id: &tg::build::Id) -> tangram_error::Result<()> {
		self.client.cancel_build(id).await
	}
}
