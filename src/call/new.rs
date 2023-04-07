use super::{Call, Data};
use crate::{error::Result, function::Function, instance::Instance, operation, value::Value};
use std::collections::BTreeMap;

impl Call {
	/// Create a new function call.
	pub async fn new(
		tg: &Instance,
		function: Function,
		env: BTreeMap<String, Value>,
		args: Vec<Value>,
	) -> Result<Self> {
		// Create the operation data.
		let function_ = function.to_data();
		let context_ = env
			.iter()
			.map(|(key, value)| (key.clone(), value.to_data()))
			.collect();
		let args_ = args.iter().map(Value::to_data).collect();
		let operation = operation::Data::Call(Data {
			function: function_,
			env: context_,
			args: args_,
		});

		// Serialize and hash the data.
		let mut bytes = Vec::new();
		operation.serialize(&mut bytes).unwrap();
		let hash = operation::Hash(crate::hash::Hash::new(&bytes));

		// Add the operation.
		let hash = tg.database.add_operation(hash, &bytes).await?;

		// Create the call.
		let call = Self {
			hash,
			function,
			env,
			args,
		};

		Ok(call)
	}
}
