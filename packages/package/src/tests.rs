use crate::lockfile::Lockfile;
use crate::ROOT_MODULE_FILE_NAME;
use async_trait::async_trait;
use futures::stream::BoxStream;
use tangram_client as tg;
use tg::{Client, WrapErr};

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

	let lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve simple_diamond case.");

	let lockfile = Lockfile::from_package(&client, client.artifact(metadata), lock)
		.await
		.unwrap();
	let lockfile = serde_json::to_string_pretty(&lockfile).unwrap();
	println!("{lockfile}");
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

	let lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve simple_backtrack case.");

	let lockfile = Lockfile::from_package(&client, client.artifact(metadata), lock)
		.await
		.unwrap();
	let lockfile = serde_json::to_string_pretty(&lockfile).unwrap();
	println!("{lockfile}");
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

	let lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve diamond_backtrack case.");

	let lockfile = Lockfile::from_package(&client, client.artifact(metadata), lock)
		.await
		.unwrap();
	let lockfile = serde_json::to_string_pretty(&lockfile).unwrap();
	println!("{lockfile}");
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
	let lock = client
		.try_solve(metadata.clone())
		.await
		.expect("Failed to solve diamond_backtrack case.");

	let lockfile = Lockfile::from_package(&client, client.artifact(metadata), lock)
		.await
		.unwrap();
	let lockfile = serde_json::to_string_pretty(&lockfile).unwrap();
	println!("{lockfile}");
}

use std::{
	collections::{BTreeMap, HashMap},
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

/// A test client for debugging the version solving algorithm without requiring a full backend.
#[derive(Debug, Clone)]
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
		entry.push(MockPackage { metadata, artifact })
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
	) -> Result<tg::Lock, crate::version::Report> {
		let package = self
			.get_package_version(
				metadata.name.as_ref().unwrap(),
				metadata.version.as_ref().unwrap(),
			)
			.await
			.unwrap()
			.unwrap()
			.into();
		crate::version::solve(self, package, BTreeMap::new())
			.await
			.unwrap()
	}

	pub fn artifact(&self, metadata: tg::package::Metadata) -> tg::Artifact {
		let state = self.state.lock().unwrap();
		state
			.packages
			.get(metadata.name.as_ref().unwrap())
			.unwrap()
			.iter()
			.find_map(|mock| (mock.metadata == metadata).then_some(mock.artifact.clone()))
			.unwrap()
	}
}

#[async_trait]
impl tg::Client for MockClient {
	fn clone_box(&self) -> Box<dyn tg::Client> {
		Box::new(self.clone())
	}

	fn downgrade_box(&self) -> Box<dyn tg::Handle> {
		unimplemented!()
	}

	fn path(&self) -> Option<&Path> {
		self.client.path()
	}

	fn file_descriptor_semaphore(&self) -> &tokio::sync::Semaphore {
		self.client.file_descriptor_semaphore()
	}

	async fn stop(&self) -> tg::Result<()> {
		self.client.stop().await
	}

	async fn status(&self) -> tg::Result<tg::status::Status> {
		self.client.status().await
	}

	async fn clean(&self) -> tg::Result<()> {
		self.client.clean().await
	}

	async fn get_object_exists(&self, id: &tg::object::Id) -> tg::Result<bool> {
		self.client.get_object_exists(id).await
	}

	async fn try_get_object(&self, id: &tg::object::Id) -> tg::Result<Option<bytes::Bytes>> {
		self.client.try_get_object(id).await
	}

	async fn try_put_object(
		&self,
		id: &tg::object::Id,
		bytes: &bytes::Bytes,
	) -> tg::Result<tg::Result<(), Vec<tg::object::Id>>> {
		self.client.try_put_object(id, bytes).await
	}

	async fn try_get_tracker(&self, path: &Path) -> tg::Result<Option<tg::Tracker>> {
		self.client.try_get_tracker(path).await
	}

	async fn set_tracker(&self, path: &Path, tracker: &tg::Tracker) -> tg::Result<()> {
		self.client.set_tracker(path, tracker).await
	}

	async fn try_get_build_for_target(
		&self,
		id: &tg::target::Id,
	) -> tg::Result<Option<tg::build::Id>> {
		self.client.try_get_build_for_target(id).await
	}

	async fn get_or_create_build_for_target(
		&self,
		id: &tg::target::Id,
	) -> tg::Result<tg::build::Id> {
		self.client.get_or_create_build_for_target(id).await
	}

	async fn try_get_build_queue_item(&self) -> tg::Result<Option<tg::build::Id>> {
		self.client.try_get_build_queue_item().await
	}

	async fn try_get_build_target(&self, id: &tg::build::Id) -> tg::Result<Option<tg::target::Id>> {
		self.client.try_get_build_target(id).await
	}

	async fn try_get_build_children(
		&self,
		id: &tg::build::Id,
	) -> tg::Result<Option<BoxStream<'static, tg::Result<tg::build::Id>>>> {
		self.client.try_get_build_children(id).await
	}

	async fn add_build_child(
		&self,
		build_id: &tg::build::Id,
		child_id: &tg::build::Id,
	) -> tg::Result<()> {
		self.client.add_build_child(build_id, child_id).await
	}

	async fn try_get_build_log(
		&self,
		id: &tg::build::Id,
	) -> tg::Result<Option<BoxStream<'static, tg::Result<bytes::Bytes>>>> {
		self.client.try_get_build_log(id).await
	}

	async fn add_build_log(&self, build_id: &tg::build::Id, bytes: bytes::Bytes) -> tg::Result<()> {
		self.client.add_build_log(build_id, bytes).await
	}

	async fn try_get_build_result(
		&self,
		id: &tg::build::Id,
	) -> tg::Result<Option<tg::Result<tg::Value>>> {
		self.client.try_get_build_result(id).await
	}

	async fn set_build_result(
		&self,
		build_id: &tg::build::Id,
		result: tg::Result<tg::Value>,
	) -> tg::Result<()> {
		self.client.set_build_result(build_id, result).await
	}

	async fn finish_build(&self, id: &tg::build::Id) -> tg::Result<()> {
		self.client.finish_build(id).await
	}
	async fn create_login(&self) -> tg::Result<tg::user::Login> {
		self.client.create_login().await
	}

	async fn get_login(&self, id: &tg::Id) -> tg::Result<Option<tg::user::Login>> {
		self.client.get_login(id).await
	}

	async fn get_current_user(&self, token: &str) -> tg::Result<Option<tg::user::User>> {
		self.client.get_current_user(token).await
	}

	async fn search_packages(&self, quer: &str) -> tg::Result<Vec<tg::package::Package>> {
		let Some(mock) = self.get_package(quer).await? else {
			return Ok(vec![]);
		};
		Ok(vec![mock])
	}

	async fn get_package(&self, name: &str) -> tg::Result<Option<tg::package::Package>> {
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
	) -> tg::Result<Option<tg::artifact::Id>> {
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

	async fn publish_package(&self, _token: &str, id: &tg::artifact::Id) -> tg::Result<()> {
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

		let module = tangram_lsp::Module::analyze(text)?;
		let metadata = module.metadata.unwrap();

		self.publish(metadata, tg::Artifact::with_id(id.clone()));
		Ok(())
	}

	async fn get_package_metadata(&self, id: &tg::Id) -> tg::Result<Option<tg::package::Metadata>> {
		self.client.get_package_metadata(id).await
	}

	async fn get_package_dependencies(
		&self,
		id: &tg::Id,
	) -> tg::Result<Option<Vec<tg::Dependency>>> {
		self.client.get_package_dependencies(id).await
	}
}
