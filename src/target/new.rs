use super::{Data, Target};
use crate::{
	block::Block, error::Result, instance::Instance, operation, path::Subpath, value::Value,
};
use itertools::Itertools;
use std::collections::BTreeMap;

impl Target {
	/// Create a target.
	pub async fn new(
		tg: &Instance,
		package: Block,
		module_path: Subpath,
		name: String,
		env: BTreeMap<String, Value>,
		args: Vec<Value>,
	) -> Result<Self> {
		// Create the data.
		let env_ = env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args_ = args.iter().map(Value::to_data).collect();
		let data = operation::Data::Target(Data {
			package,
			module_path: module_path.clone(),
			name: name.clone(),
			env: env_,
			args: args_,
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Collect the children.
		let children = Some(package)
			.into_iter()
			.chain(env.values().flat_map(Value::blocks))
			.chain(args.iter().flat_map(Value::blocks))
			.collect_vec();

		// Create the block.
		let block = Block::new(tg, children, &data).await?;

		// Create the target.
		let target = Self {
			block,
			package,
			module_path,
			name,
			env,
			args,
		};

		Ok(target)
	}
}
