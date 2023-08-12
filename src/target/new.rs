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
		path: Subpath,
		name: String,
		env: BTreeMap<String, Value>,
		args: Vec<Value>,
	) -> Result<Self> {
		// Collect the children.
		let children = Some(package.clone())
			.into_iter()
			.chain(env.values().flat_map(Value::blocks))
			.chain(args.iter().flat_map(Value::blocks))
			.collect_vec();

		// Create the data.
		let env_ = env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args_ = args.iter().map(Value::to_data).collect();
		let data = operation::Data::Target(Data {
			package: package.id(),
			path: path.clone(),
			name: name.clone(),
			env: env_,
			args: args_,
		});

		// Serialize the data.
		let mut bytes = Vec::new();
		data.serialize(&mut bytes).unwrap();
		let data = bytes;

		// Create the block.
		let block = Block::with_children_and_data(children, &data)?;

		// Create the target.
		let target = Self {
			block,
			package,
			path,
			name,
			env,
			args,
		};

		Ok(target)
	}
}
