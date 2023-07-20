use super::{Data, Task};
use crate::{
	artifact::Artifact, block::Block, checksum::Checksum, error::Result, instance::Instance,
	operation, system::System, template::Template,
};
use itertools::Itertools;
use std::collections::BTreeMap;

impl Task {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		tg: &Instance,
		system: System,
		executable: Template,
		env: BTreeMap<String, Template>,
		args: Vec<Template>,
		checksum: Option<Checksum>,
		unsafe_: bool,
		network: bool,
	) -> Result<Self> {
		// Create the operation data.
		let executable_ = executable.to_data();
		let env_ = env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args_ = args.iter().map(Template::to_data).collect();
		let data = operation::Data::Task(Data {
			system,
			executable: executable_,
			env: env_,
			args: args_,
			checksum: checksum.clone(),
			unsafe_,
			network,
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Collect the children.
		let children = executable
			.artifacts()
			.chain(env.values().flat_map(Template::artifacts))
			.chain(args.iter().flat_map(Template::artifacts))
			.map(Artifact::block)
			.collect_vec();

		// Create the block.
		let block = Block::new(tg, children, &data)?;

		// Create the task.
		let task = Self {
			block,
			system,
			executable,
			env,
			args,
			checksum,
			unsafe_,
			network,
		};

		Ok(task)
	}
}
