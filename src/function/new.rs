use super::{Data, Function};
use crate::{
	error::Result, instance::Instance, operation, package::Package, path::Subpath, value::Value,
};
use std::collections::BTreeMap;

impl Function {
	/// Create a new function call.
	pub async fn new(
		tg: &Instance,
		package: Package,
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
		let operation = operation::Data::Function(Data {
			package_hash: package.hash(),
			module_path: module_path.clone(),
			name: name.clone(),
			env: env_,
			args: args_,
		});

		// Serialize and hash the data.
		let mut bytes = Vec::new();
		operation.serialize(&mut bytes).unwrap();
		let hash = operation::Hash(crate::hash::Hash::new(&bytes));

		// Add the operation.
		let hash = tg.database.add_operation(hash, &bytes).await?;

		// Create the function.
		let function = Self {
			hash,
			package_hash: package.hash(),
			module_path,
			name,
			env,
			args,
		};

		Ok(function)
	}
}
