use super::{Data, Process};
use crate::{
	checksum::Checksum, error::Result, instance::Instance, operation, system::System,
	template::Template,
};
use std::collections::BTreeMap;

impl Process {
	#[allow(clippy::too_many_arguments)]
	pub async fn new(
		tg: &Instance,
		system: System,
		executable: Template,
		env: BTreeMap<String, Template>,
		args: Vec<Template>,
		checksum: Option<Checksum>,
		unsafe_: bool,
		network: bool,
		host_paths: Vec<String>,
	) -> Result<Self> {
		// Create the operation data.
		let executable_ = executable.to_data();
		let env_ = env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args_ = args.iter().map(Template::to_data).collect();
		let data = operation::Data::Process(Data {
			system,
			executable: executable_,
			env: env_,
			args: args_,
			checksum: checksum.clone(),
			unsafe_,
			network,
			host_paths: host_paths.clone(),
		});

		// Serialize and hash the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let hash = operation::Hash(crate::hash::Hash::new(&bytes));

		// Add the operation.
		tg.database.add_operation(hash, &bytes).await?;

		// Create the process.
		let process = Self {
			hash,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe_,
			network,
			host_paths,
		};

		Ok(process)
	}
}
