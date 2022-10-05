use crate::Cli;
use anyhow::Result;
use std::collections::BTreeMap;
use tangram_core::{hash::Hash, system::System};

impl Cli {
	pub async fn create_target_args(&self, system: Option<System>) -> Result<Hash> {
		let builder = self.builder.lock_shared().await?;
		let mut target_arg = BTreeMap::new();
		let system = if let Some(system) = system {
			system
		} else {
			System::host()?
		};
		let system = builder
			.add_expression(&tangram_core::expression::Expression::String(
				system.to_string().into(),
			))
			.await?;
		target_arg.insert("system".into(), system);
		let target_arg = builder
			.add_expression(&tangram_core::expression::Expression::Map(target_arg))
			.await?;
		let target_args = vec![target_arg];
		let target_args = builder
			.add_expression(&tangram_core::expression::Expression::Array(target_args))
			.await?;
		Ok(target_args)
	}
}
